use chrono::{DateTime, Utc};

use crate::common::send_email::GoogleSendEmailRequest;

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
            "METHOD:REQUEST".to_string(),
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
        lines.push("STATUS:CONFIRMED".to_string());
        lines.push("SEQUENCE:0".to_string());
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
        let boundary = format!("rust-gmail-{}", Utc::now().timestamp_nanos());
        let ics = event.to_ics(from, to);
        let message = format!(
            "From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\nMIME-Version: 1.0\r\n\
             Content-Type: multipart/alternative; boundary=\"{boundary}\"\r\n\r\n\
             --{boundary}\r\nContent-Type: text/plain; charset=\"UTF-8\"\r\n\r\n{content}\r\n\
             --{boundary}\r\nContent-Type: text/calendar; charset=\"UTF-8\"; method=REQUEST\r\n\r\n{ics}\
             --{boundary}--\r\n",
        );
        Self::from_raw(&message)
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
