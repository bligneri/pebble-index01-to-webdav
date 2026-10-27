//! Pebble Index 01 webhook, protocol v1 (sdk-docs source/index-01/webhooks.md).
use std::{collections::HashMap, convert::Infallible, sync::Arc, sync::Mutex};

use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::{
    classify::{Kind, classify},
    config::JournalMode,
    ics,
};

/// Accepted clock difference and how long delivery IDs are remembered, in seconds.
const WINDOW: i64 = 300;

type Reject = (StatusCode, &'static str);
const BAD_SIGNATURE: Reject = (StatusCode::UNAUTHORIZED, "bad signature");

/// Delivery IDs already accepted, with their expiry.
// ponytail: in memory, 1 replica; a shared store with a TTL if it ever scales out.
#[derive(Default)]
pub struct Seen(Mutex<HashMap<String, i64>>);

impl Seen {
    fn first(&self, id: &str, ts: i64, now: i64) -> bool {
        let mut seen = self.0.lock().unwrap();
        seen.retain(|_, expiry| *expiry >= now);
        if seen.contains_key(id) {
            return false;
        }
        seen.insert(id.to_string(), now.max(ts) + WINDOW);
        true
    }
}

pub async fn hook(
    State(st): State<Arc<crate::State>>,
    h: HeaderMap,
    body: Bytes,
) -> (StatusCode, &'static str) {
    let header = |k: &str| {
        h.get(k)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-")
            .to_owned()
    };
    let (trigger, delivery) = (header("x-index-trigger"), header("x-index-delivery"));
    let (status, reason) = handle(&st, &h, body).await;
    if status.is_success() {
        tracing::info!(%trigger, %delivery, status = status.as_u16(), reason, "request");
    } else {
        tracing::warn!(%trigger, %delivery, status = status.as_u16(), reason, "request");
    }
    (status, reason)
}

async fn handle(st: &crate::State, h: &HeaderMap, body: Bytes) -> (StatusCode, &'static str) {
    let now = Utc::now();
    let (test, delivery) = match verify(
        st.secrets.webhook.as_bytes(),
        h,
        &body,
        now.timestamp(),
        &st.seen,
    ) {
        Ok(v) => v,
        Err(reject) => return reject,
    };
    if test {
        return (StatusCode::OK, "test event ok");
    }
    let (text, recorded_ms) = match fields(h, body).await {
        Ok(f) => f,
        Err(reject) => return reject,
    };
    let recorded = recorded_ms
        .and_then(DateTime::from_timestamp_millis)
        .unwrap_or(now)
        .with_timezone(&st.cfg.timezone);
    let c = classify(&st.cfg, &text);
    if c.text.is_empty() {
        return (StatusCode::BAD_REQUEST, "empty text after keywords");
    }
    let collection = &st.cfg.domains[c.domain].collection;
    tracing::info!(kind = ?c.kind, domain = c.domain, collection, %recorded, "classified");
    let Some(dav) = &st.dav else {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            "CalDAV password not configured",
        );
    };

    let written = match c.kind {
        Kind::Journal if st.cfg.journal.mode == JournalMode::New => {
            let uid = format!("pebble-{}", safe_id(&delivery));
            let ics = ics::journal_entry(&uid, now, recorded.to_utc(), c.text);
            dav.create(collection, &uid, ics).await
        }
        Kind::Journal => {
            let day = recorded.date_naive();
            let uid = format!("pebble-journal-{}-{day}", c.domain);
            let line = format!("{} {}", recorded.format("%H:%M"), c.text);
            dav.append_journal(collection, &uid, day, &line, now).await
        }
        kind => {
            // The delivery ID is stable per recording: a redelivery hits the same UID instead of a duplicate.
            let uid = format!("pebble-{}", safe_id(&delivery));
            let ics = ics::note_or_todo(kind == Kind::Todo, &uid, now, c.text);
            dav.create(collection, &uid, ics).await
        }
    };
    match written {
        Ok(()) => (StatusCode::OK, "stored"),
        Err(e) => {
            tracing::error!(error = e, collection, "CalDAV write failed");
            (StatusCode::BAD_GATEWAY, "CalDAV write failed")
        }
    }
}

/// Delivery ID reduced to characters safe in a CalDAV resource name.
fn safe_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .collect()
}

/// Checks version, signature, timestamp window and replay. Returns whether it is a test event, and the delivery ID.
pub fn verify(
    secret: &[u8],
    h: &HeaderMap,
    body: &[u8],
    now: i64,
    seen: &Seen,
) -> Result<(bool, String), Reject> {
    let get = |k: &str| h.get(k).and_then(|v| v.to_str().ok());
    if get("x-index-webhook-version") != Some("1") {
        return Err((StatusCode::BAD_REQUEST, "unsupported webhook version"));
    }
    let (Some(ts_raw), Some(delivery), Some(trigger), Some(sig)) = (
        get("x-index-timestamp"),
        get("x-index-delivery"),
        get("x-index-trigger"),
        get("x-index-signature"),
    ) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "unsigned request: turn on Sign requests",
        ));
    };
    let ts: i64 = ts_raw
        .parse()
        .map_err(|_| (StatusCode::BAD_REQUEST, "bad timestamp"))?;
    let test = get("x-index-test") == Some("true");

    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC takes any key size");
    mac.update(format!("v1\n{ts_raw}\n{delivery}\n{trigger}\n{}\n", test as u8).as_bytes());
    mac.update(body);
    // verify_slice compares in constant time.
    mac.verify_slice(&unhex(sig).ok_or(BAD_SIGNATURE)?)
        .map_err(|_| BAD_SIGNATURE)?;

    if (now - ts).abs() > WINDOW {
        return Err((
            StatusCode::UNAUTHORIZED,
            "timestamp outside the 5 minute window",
        ));
    }
    if !seen.first(delivery, ts, now) {
        return Err((StatusCode::CONFLICT, "duplicate delivery"));
    }
    Ok((test, delivery.to_string()))
}

/// Transcription and recordedAt (Unix ms) from the multipart body; audio is ignored.
pub async fn fields(h: &HeaderMap, body: Bytes) -> Result<(String, Option<i64>), Reject> {
    let bad = |m| (StatusCode::BAD_REQUEST, m);
    let ct = h
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .ok_or(bad("no content type"))?;
    let boundary = multer::parse_boundary(ct).map_err(|_| bad("not multipart/form-data"))?;
    let stream = futures_util::stream::once(async move { Ok::<_, Infallible>(body) });
    let mut mp = multer::Multipart::new(stream, boundary);

    let (mut text, mut at) = (None, None);
    while let Some(field) = mp
        .next_field()
        .await
        .map_err(|_| bad("bad multipart body"))?
    {
        match field.name() {
            Some("transcription") => {
                text = Some(field.text().await.map_err(|_| bad("bad transcription"))?)
            }
            Some("recordedAt") => at = field.text().await.ok().and_then(|t| t.trim().parse().ok()),
            _ => {}
        }
    }
    let text = text.ok_or(bad("no transcription: set What to send to Transcription"))?;
    Ok((text, at))
}

fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.is_ascii() || !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;

    // Worked example of the SDK docs.
    const SECRET: &[u8] = b"0123456789abcdef0123456789abcdef";
    const TS: i64 = 1791244800;
    const B: &str = "7d3b8e5a-1f2c-4a6b-9c0d-2e4f6a8b0c1d";

    fn body() -> String {
        let part = |name: &str, value: &str| {
            format!("--{B}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
        };
        part("transcription", "Index webhook test event")
            + &part("test", "true")
            + &part("recordedAt", "1791244798766")
            + &part("client", "ring")
            + &format!("--{B}--\r\n")
    }

    fn headers() -> HeaderMap {
        let mut h = HeaderMap::new();
        for (k, v) in [
            ("x-index-webhook-version", "1"),
            ("x-index-trigger", "test-event"),
            ("x-index-test", "true"),
            ("x-index-timestamp", "1791244800"),
            ("x-index-delivery", "6f1c2a3e-8b4d-4c5e-9f60-1a2b3c4d5e6f"),
            (
                "x-index-signature",
                "f3cda81969d98bfdaf2ed8e5e4019a8c3f2b99263e39a8c090c48d63c51738f7",
            ),
            (
                "content-type",
                "multipart/form-data; boundary=7d3b8e5a-1f2c-4a6b-9c0d-2e4f6a8b0c1d",
            ),
        ] {
            h.insert(k, v.parse().unwrap());
        }
        h
    }

    fn code(r: Result<(bool, String), Reject>) -> StatusCode {
        r.map_or_else(|e| e.0, |_| StatusCode::OK)
    }

    #[test]
    fn worked_example_verifies() {
        let body = body();
        assert_eq!(body.len(), 460);
        let digest = sha2::Sha256::digest(body.as_bytes());
        assert_eq!(
            unhex("dd947b94767662935ea27baa1e7e4297d7cc86513f66ff2d96b3c975b39b8d05").unwrap(),
            digest.to_vec()
        );
        assert_eq!(
            verify(SECRET, &headers(), body.as_bytes(), TS, &Seen::default()),
            Ok((true, "6f1c2a3e-8b4d-4c5e-9f60-1a2b3c4d5e6f".to_string()))
        );
    }

    #[test]
    fn rejects_tampering_staleness_replay_and_unsigned() {
        let (h, body) = (headers(), body());
        let ok = |secret, body: &str, now| {
            code(verify(secret, &h, body.as_bytes(), now, &Seen::default()))
        };
        assert_eq!(
            ok(SECRET, &body.replace("test event", "test evenT"), TS),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(ok(b"other secret", &body, TS), StatusCode::UNAUTHORIZED);
        assert_eq!(ok(SECRET, &body, TS + WINDOW + 1), StatusCode::UNAUTHORIZED);
        assert_eq!(ok(SECRET, &body, TS - WINDOW), StatusCode::OK);

        let seen = Seen::default();
        assert_eq!(
            code(verify(SECRET, &h, body.as_bytes(), TS, &seen)),
            StatusCode::OK
        );
        assert_eq!(
            code(verify(SECRET, &h, body.as_bytes(), TS + 10, &seen)),
            StatusCode::CONFLICT
        );

        let mut unsigned = h.clone();
        unsigned.remove("x-index-signature");
        assert_eq!(
            code(verify(SECRET, &unsigned, body.as_bytes(), TS, &seen)),
            StatusCode::UNAUTHORIZED
        );
        let mut old = h.clone();
        old.remove("x-index-webhook-version");
        assert_eq!(
            code(verify(SECRET, &old, body.as_bytes(), TS, &seen)),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn safe_id_keeps_uuid_drops_path_chars() {
        assert_eq!(
            safe_id("6f1c2a3e-8b4d-4c5e-9f60-1a2b3c4d5e6f"),
            "6f1c2a3e-8b4d-4c5e-9f60-1a2b3c4d5e6f"
        );
        assert_eq!(safe_id("../x?y z"), "xyz");
    }

    #[test]
    fn seen_forgets_after_window() {
        let seen = Seen::default();
        assert!(seen.first("a", TS, TS));
        assert!(!seen.first("a", TS, TS + WINDOW));
        assert!(seen.first("a", TS, TS + WINDOW + 1));
    }

    #[tokio::test]
    async fn reads_fields() {
        let got = fields(&headers(), Bytes::from(body())).await;
        assert_eq!(
            got,
            Ok(("Index webhook test event".into(), Some(1791244798766)))
        );
        let no_text = body().replacen("name=\"transcription\"", "name=\"other\"", 1);
        assert_eq!(
            fields(&headers(), Bytes::from(no_text))
                .await
                .unwrap_err()
                .0,
            StatusCode::BAD_REQUEST
        );
    }
}
