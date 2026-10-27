//! Hand-written iCalendar (RFC 5545): only the few properties jtx Board needs.
use chrono::{DateTime, NaiveDate, Utc};

/// Note (VJOURNAL without date) or todo (VTODO).
pub fn note_or_todo(todo: bool, uid: &str, now: DateTime<Utc>, text: &str) -> String {
    let (component, mut props) = if todo {
        ("VTODO", vec!["STATUS:NEEDS-ACTION".to_string()])
    } else {
        ("VJOURNAL", vec![])
    };
    props.extend([
        format!("SUMMARY:{}", escape(&summary(text))),
        format!("DESCRIPTION:{}", escape(text)),
    ]);
    calendar(component, uid, now, props)
}

/// New day journal holding its first line.
pub fn journal(uid: &str, now: DateTime<Utc>, day: NaiveDate, line: &str) -> String {
    let props = vec![
        format!("DTSTART;VALUE=DATE:{}", day.format("%Y%m%d")),
        format!("SUMMARY:Journal {day}"),
        format!("DESCRIPTION:{}", escape(line)),
    ];
    calendar("VJOURNAL", uid, now, props)
}

/// Appends a line to the DESCRIPTION of an existing journal, keeping everything else as the server sent it.
pub fn append(ics: &str, now: DateTime<Utc>, line: &str) -> String {
    let unfolded = ics
        .replace("\r\n", "\n")
        .replace("\n ", "")
        .replace("\n\t", "");
    let mut out = Vec::new();
    let mut done = false;
    for l in unfolded.lines() {
        if l.starts_with("LAST-MODIFIED") {
            continue;
        }
        if !done && (l.starts_with("DESCRIPTION:") || l.starts_with("DESCRIPTION;")) {
            let sep = if l.ends_with(':') { "" } else { "\\n" };
            out.push(format!("{l}{sep}{}", escape(line)));
            done = true;
            continue;
        }
        if l == "END:VJOURNAL" {
            if !done {
                out.push(format!("DESCRIPTION:{}", escape(line)));
                done = true;
            }
            out.push(format!("LAST-MODIFIED:{}", stamp(now)));
        }
        out.push(l.to_string());
    }
    out.iter().map(|l| fold(l) + "\r\n").collect()
}

/// First sentence, at most 60 characters.
pub fn summary(text: &str) -> String {
    let first = text
        .split_inclusive(['.', '!', '?', '\n'])
        .next()
        .unwrap_or("");
    first
        .trim()
        .trim_end_matches('.')
        .chars()
        .take(60)
        .collect()
}

fn calendar(component: &str, uid: &str, now: DateTime<Utc>, props: Vec<String>) -> String {
    let head = [
        "BEGIN:VCALENDAR".to_string(),
        "VERSION:2.0".into(),
        "PRODID:-//pebble-index01-to-webdav//FR".into(),
        format!("BEGIN:{component}"),
        format!("UID:{uid}"),
        format!("DTSTAMP:{}", stamp(now)),
        "CATEGORIES:Pebble".into(),
    ];
    let tail = [format!("END:{component}"), "END:VCALENDAR".into()];
    head.into_iter()
        .chain(props)
        .chain(tail)
        .map(|l| fold(&l) + "\r\n")
        .collect()
}

fn stamp(t: DateTime<Utc>) -> String {
    t.format("%Y%m%dT%H%M%SZ").to_string()
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace("\r\n", "\\n")
        .replace('\n', "\\n")
}

/// Lines of at most 75 octets, never splitting a UTF-8 character.
fn fold(line: &str) -> String {
    let mut out = String::with_capacity(line.len() + 8);
    let mut len = 0;
    for c in line.chars() {
        if len + c.len_utf8() > 75 {
            out.push_str("\r\n ");
            len = 1;
        }
        out.push(c);
        len += c.len_utf8();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1791244800, 0).unwrap()
    }

    #[test]
    fn note_and_todo() {
        let note = note_or_todo(false, "u1", now(), "Idée: lait, pain; œufs.\nSuite");
        assert_eq!(
            note,
            "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//pebble-index01-to-webdav//FR\r\nBEGIN:VJOURNAL\r\n\
             UID:u1\r\nDTSTAMP:20261006T000000Z\r\nCATEGORIES:Pebble\r\n\
             SUMMARY:Idée: lait\\, pain\\; œufs\r\nDESCRIPTION:Idée: lait\\, pain\\; œufs.\\nSuite\r\n\
             END:VJOURNAL\r\nEND:VCALENDAR\r\n"
        );
        let todo = note_or_todo(true, "u2", now(), "Appeler");
        assert!(todo.contains("BEGIN:VTODO\r\n") && todo.contains("STATUS:NEEDS-ACTION\r\n"));
    }

    #[test]
    fn folds_at_75_octets_on_char_boundaries() {
        let long = format!("DESCRIPTION:{}", "é".repeat(100));
        let folded = fold(&long);
        assert!(folded.split("\r\n").all(|l| l.len() <= 75));
        assert_eq!(folded.replace("\r\n ", ""), long);
    }

    #[test]
    fn journal_then_append() {
        let day = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let first = journal("j1", now(), day, "08:00 réveil");
        assert!(first.contains("DTSTART;VALUE=DATE:20261005\r\nSUMMARY:Journal 2026-10-05\r\n"));

        let later = now() + chrono::Duration::hours(2);
        let second = append(&first, later, "10:00 réunion, budget");
        assert!(second.contains("DESCRIPTION:08:00 réveil\\n10:00 réunion\\, budget\r\n"));
        assert!(second.contains("LAST-MODIFIED:20261006T020000Z\r\nEND:VJOURNAL\r\n"));
        // Appending again replaces LAST-MODIFIED instead of adding one.
        assert_eq!(
            append(&second, later, "x").matches("LAST-MODIFIED").count(),
            1
        );
    }

    #[test]
    fn append_to_server_copy() {
        // Folded with LF and a tab, DESCRIPTION with a parameter, as another client may write it.
        let ics = "BEGIN:VCALENDAR\nBEGIN:VJOURNAL\nUID:j\nDESCRIPTION;LANGUAGE=fr:un\n\tdeux\nEND:VJOURNAL\nEND:VCALENDAR\n";
        assert!(append(ics, now(), "trois").contains("DESCRIPTION;LANGUAGE=fr:undeux\\ntrois\r\n"));
        // Description removed in jtx Board.
        let bare =
            "BEGIN:VCALENDAR\r\nBEGIN:VJOURNAL\r\nUID:j\r\nEND:VJOURNAL\r\nEND:VCALENDAR\r\n";
        assert!(append(bare, now(), "trois").contains("DESCRIPTION:trois\r\nLAST-MODIFIED:"));
    }

    #[test]
    fn summary_is_first_sentence_capped() {
        assert_eq!(summary("Acheter du pain. Et du lait"), "Acheter du pain");
        assert_eq!(summary(&"a".repeat(80)).len(), 60);
        assert_eq!(summary(""), "");
    }
}
