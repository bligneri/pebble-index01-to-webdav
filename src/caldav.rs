//! CalDAV writes (RFC 4791): plain PUTs, conditional on ETags.
use std::time::Duration;

use chrono::{DateTime, NaiveDate, Utc};
use reqwest::{StatusCode, header};

use crate::{config::Config, ics};

pub struct Dav {
    http: reqwest::Client,
    base: String,
    user: String,
    password: String,
}

impl Dav {
    pub fn new(cfg: &Config, user: &str, password: &str) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("TLS backend");
        Self {
            http,
            base: cfg.caldav.base_url.clone(),
            user: user.into(),
            password: password.into(),
        }
    }

    fn url(&self, collection: &str, uid: &str) -> String {
        format!("{}{collection}/{uid}.ics", self.base)
    }

    async fn put(
        &self,
        url: &str,
        ics: String,
        condition: (header::HeaderName, &str),
    ) -> Result<StatusCode, String> {
        let res = self
            .http
            .put(url)
            .basic_auth(&self.user, Some(&self.password))
            .header(header::CONTENT_TYPE, "text/calendar; charset=utf-8")
            .header(condition.0, condition.1)
            .body(ics)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        Ok(res.status())
    }

    /// New note or todo. An existing UID (same delivery seen again) counts as stored.
    pub async fn create(&self, collection: &str, uid: &str, ics: String) -> Result<(), String> {
        match self
            .put(
                &self.url(collection, uid),
                ics,
                (header::IF_NONE_MATCH, "*"),
            )
            .await?
        {
            s if s.is_success() || s == StatusCode::PRECONDITION_FAILED => Ok(()),
            s => Err(format!("PUT {s}")),
        }
    }

    /// Adds a line to the day's journal, creating it on the first line of the day.
    pub async fn append_journal(
        &self,
        collection: &str,
        uid: &str,
        day: NaiveDate,
        line: &str,
        now: DateTime<Utc>,
    ) -> Result<(), String> {
        let url = self.url(collection, uid);
        // A 412 means another write got in between: read again and retry.
        for _ in 0..3 {
            let res = self
                .http
                .get(&url)
                .basic_auth(&self.user, Some(&self.password))
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = match res.status() {
                StatusCode::NOT_FOUND => {
                    let ics = ics::journal(uid, now, day, line);
                    self.put(&url, ics, (header::IF_NONE_MATCH, "*")).await?
                }
                s if s.is_success() => {
                    let etag = res
                        .headers()
                        .get(header::ETAG)
                        .and_then(|v| v.to_str().ok())
                        .map(str::to_owned);
                    let etag = etag.ok_or("GET returned no ETag")?;
                    let ics = ics::append(&res.text().await.map_err(|e| e.to_string())?, now, line);
                    self.put(&url, ics, (header::IF_MATCH, &etag)).await?
                }
                s => return Err(format!("GET {s}")),
            };
            match status {
                s if s.is_success() => return Ok(()),
                StatusCode::PRECONDITION_FAILED => continue,
                s => return Err(format!("PUT {s}")),
            }
        }
        Err("journal changed 3 times in a row".into())
    }
}
