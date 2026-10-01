use chrono::{DateTime, Utc};

use crate::common::send_email::{format_message, wrapped_base64, GoogleSendEmailRequest};

/// The iCalendar method of a [`CalendarEvent`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalendarMethod {
    /// Invite recipients to the event, or update it if it was sent before. This is the default.
    #[default]
    Request,
    /// Cancel an event that was sent before. The `uid` and `sequence` should match
    /// (or be higher than) the event being cancelled.
    Cancel,
}

impl CalendarMethod {
    fn as_str(&self) -> &'static str {
        match self {
            CalendarMethod::Request => "REQUEST",
            CalendarMethod::Cancel => "CANCEL",
        }
    }
}

/// A calendar event that can be sent as an iCalendar invite using
/// [`GmailClient::send_calendar_event`](crate::GmailClient::send_calendar_event).
#[derive(Debug, Clone)]
pub struct CalendarEvent {
    summary: String,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    description: Option<String>,
    location: Option<String>,
    uid: Option<String>,
    sequence: u32,
    method: CalendarMethod,
}

impl CalendarEvent {
    /// Create a new event with a `summary` (title) that runs from `start` to `end`.
    pub fn new<S: Into<String>>(summary: S, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            summary: summary.into(),
            start,
            end,
            description: None,
            location: None,
            uid: None,
            sequence: 0,
            method: CalendarMethod::Request,
        }
    }

    /// Set the description of the event.
    pub fn description<S: Into<String>>(mut self, description: S) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the location of the event.
    pub fn location<S: Into<String>>(mut self, location: S) -> Self {
        self.location = Some(location.into());
        self
    }

    /// Set the unique id of the event. Re-sending an event with the same uid updates
    /// the existing event in the recipient's calendar instead of creating a new one.
    /// If not set, a uid is generated.
    pub fn uid<S: Into<String>>(mut self, uid: S) -> Self {
        self.uid = Some(uid.into());
        self
    }

    /// Set the revision number of the event, `0` by default. To update an event that was
    /// already sent, resend it with the same [`uid`](Self::uid) and a higher sequence number;
    /// calendar clients typically ignore updates that don't increase it.
    pub fn sequence(mut self, sequence: u32) -> Self {
        self.sequence = sequence;
        self
    }

    /// Set the [`CalendarMethod`], [`CalendarMethod::Request`] by default.
    /// Use [`CalendarMethod::Cancel`] together with the original [`uid`](Self::uid) to cancel an event.
    pub fn method(mut self, method: CalendarMethod) -> Self {
        self.method = method;
        self
    }

    pub(crate) fn to_ics(&self, organizer: &str, attendee: &str) -> String {
        let now = Utc::now();
        let uid = self.uid.clone().unwrap_or_else(|| {
            let domain = organizer.rsplit('@').next().unwrap_or("localhost");
            format!(
                "{}-{}@{}",
                now.timestamp_nanos(),
                self.start.timestamp(),
                domain
            )
        });

        let mut lines = vec![
            "BEGIN:VCALENDAR".to_string(),
            "VERSION:2.0".to_string(),
            "PRODID:-//rust-gmail//EN".to_string(),
            "CALSCALE:GREGORIAN".to_string(),
            format!("METHOD:{}", self.method.as_str()),
            "BEGIN:VEVENT".to_string(),
            format!("UID:{}", escape_text(&uid)),
            format!("DTSTAMP:{}", format_time(&now)),
            format!("DTSTART:{}", format_time(&self.start)),
            format!("DTEND:{}", format_time(&self.end)),
            format!("SUMMARY:{}", escape_text(&self.summary)),
        ];
        if let Some(description) = &self.description {
            lines.push(format!("DESCRIPTION:{}", escape_text(description)));
        }
        if let Some(location) = &self.location {
            lines.push(format!("LOCATION:{}", escape_text(location)));
        }
        lines.push(format!("ORGANIZER:mailto:{}", organizer));
        lines.push(format!(
            "ATTENDEE;ROLE=REQ-PARTICIPANT;PARTSTAT=NEEDS-ACTION;RSVP=TRUE:mailto:{}",
            attendee
        ));
        let status = match self.method {
            CalendarMethod::Request => "CONFIRMED",
            CalendarMethod::Cancel => "CANCELLED",
        };
        lines.push(format!("STATUS:{}", status));
        lines.push(format!("SEQUENCE:{}", self.sequence));
        lines.push("END:VEVENT".to_string());
        lines.push("END:VCALENDAR".to_string());

        lines
            .iter()
            .map(|line| fold_line(line))
            .collect::<Vec<_>>()
            .join("\r\n")
            + "\r\n"
    }
}

fn format_time(time: &DateTime<Utc>) -> String {
    time.format("%Y%m%dT%H%M%SZ").to_string()
}

fn escape_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace("\r\n", "\\n")
        .replace('\n', "\\n")
}

/// Fold a content line so that no line exceeds 75 octets (RFC 5545 3.1).
fn fold_line(line: &str) -> String {
    let mut out = String::new();
    let mut current_len = 0;
    for c in line.chars() {
        let c_len = c.len_utf8();
        if current_len + c_len > 75 {
            out.push_str("\r\n ");
            current_len = 1;
        }
        out.push(c);
        current_len += c_len;
    }
    out
}

impl GoogleSendEmailRequest {
    pub fn new_calendar_event(
        from: &str,
        to: &str,
        subject: &str,
        content: &str,
        event: &CalendarEvent,
    ) -> Self {
        let now = Utc::now().timestamp_nanos();
        let mixed = format!("rust-gmail-mixed-{}", now);
        let alt = format!("rust-gmail-alt-{}", now);
        let ics = wrapped_base64(event.to_ics(from, to).as_bytes());
        let text = wrapped_base64(content.as_bytes());
        let text_part = format_message(
            &[
                ("Content-Type", "text/plain; charset=\"UTF-8\""),
                ("Content-Transfer-Encoding", "base64"),
            ],
            &text,
        );
        let calendar_part = format_message(
            &[
                (
                    "Content-Type",
                    &format!(
                        "text/calendar; charset=\"UTF-8\"; method={}",
                        event.method.as_str()
                    ),
                ),
                ("Content-Transfer-Encoding", "base64"),
            ],
            &ics,
        );
        let alternative = format_message(
            &[(
                "Content-Type",
                &format!("multipart/alternative; boundary=\"{alt}\""),
            )],
            &format!("--{alt}\r\n{text_part}\r\n--{alt}\r\n{calendar_part}\r\n--{alt}--\r\n"),
        );
        let attachment = format_message(
            &[
                ("Content-Type", "application/ics; name=\"invite.ics\""),
                ("Content-Disposition", "attachment; filename=\"invite.ics\""),
                ("Content-Transfer-Encoding", "base64"),
            ],
            &ics,
        );
        let body =
            format!("--{mixed}\r\n{alternative}\r\n--{mixed}\r\n{attachment}\r\n--{mixed}--\r\n");
        Self::from_headers(
            &[
                ("From", from),
                ("To", to),
                ("Subject", subject),
                ("MIME-Version", "1.0"),
                (
                    "Content-Type",
                    &format!("multipart/mixed; boundary=\"{mixed}\""),
                ),
            ],
            &body,
        )
    }
}

pub fn mock_print_calendar_event(
    receiver_email: &str,
    subject: &str,
    content: &str,
    send_from_email: &str,
    event: &CalendarEvent,
) {
    println!(
        "MOCK MODE SEND CALENDAR EVENT
    Sending from {} to {}
    Subject: {}
    Content: {}
    Event:
{}",
        send_from_email,
        receiver_email,
        subject,
        content,
        event.to_ics(send_from_email, receiver_email)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn calendar_message_structure() {
        let start = Utc.ymd(2030, 1, 1).and_hms(10, 0, 0);
        let event = CalendarEvent::new("Sync, now", start, start + chrono::Duration::hours(1));
        let ics = event.to_ics("a@x.test", "b@y.test");
        assert!(ics.contains("METHOD:REQUEST\r\n"));
        assert!(ics.contains("SUMMARY:Sync\\, now\r\n"));
        assert!(ics.contains("DTSTART:20300101T100000Z\r\n"));
        assert!(ics.contains("SEQUENCE:0\r\n"));
        assert!(ics.ends_with("END:VCALENDAR\r\n"));

        let cancel = event
            .clone()
            .uid("abc")
            .sequence(2)
            .method(CalendarMethod::Cancel)
            .to_ics("a@x.test", "b@y.test");
        assert!(cancel.contains("METHOD:CANCEL\r\n"));
        assert!(cancel.contains("STATUS:CANCELLED\r\n"));
        assert!(cancel.contains("SEQUENCE:2\r\n"));
        assert!(cancel.contains("UID:abc\r\n"));
    }
}
