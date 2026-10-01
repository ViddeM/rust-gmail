use serde::{Deserialize, Serialize};

// Note: the `/me/` is a parameter for the CLIENT_ID
pub const SEND_EMAIL_ENDPOINT: &str =
    "https://gmail.googleapis.com/gmail/v1/users/me/messages/send";
pub const SEND_EMAIL_QUERY_PARAMETERS: [(&str, &str); 2] =
    [("alt", "json"), ("prettyPrint", "false")];

#[derive(Serialize, Deserialize)]
pub struct GoogleSendEmailRequest {
    raw: String,
}

pub type Headers<'a> = [(&'a str, &'a str)];

/// Format a message or MIME part as `Name: value` header lines, a blank line, then the body.
/// CR/LF in header names and values are stripped to prevent header injection.
pub fn format_message(headers: &Headers, body: &str) -> String {
    let mut message = String::new();
    for (name, value) in headers {
        let clean = |s: &str| s.replace(['\r', '\n'], "");
        message.push_str(&format!("{}: {}\r\n", clean(name), clean(value)));
    }
    message.push_str("\r\n");
    message.push_str(body);
    message
}

impl GoogleSendEmailRequest {
    pub fn new(from: &str, to: &str, subject: &str, content: &str) -> Self {
        Self::from_headers(
            &[("From", from), ("To", to), ("Subject", subject)],
            &format!("{}\r\n", content),
        )
    }

    /// Build a request from an already complete raw message (headers and body in one string).
    pub fn from_raw(message: &str) -> Self {
        Self {
            raw: base64::encode_config(message, base64::URL_SAFE),
        }
    }

    /// Build a request from `headers` and a `body`, which are kept separate until encoding.
    pub fn from_headers(headers: &Headers, body: &str) -> Self {
        Self::from_raw(&format_message(headers, body))
    }
}

#[derive(Serialize, Deserialize)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
pub struct GoogleSendEmailResponse {
    id: String,
    thread_id: String,
    label_ids: Vec<String>,
}

pub fn mock_print_email(receiver_email: &str, subject: &str, content: &str, send_from_email: &str) {
    println!(
        "MOCK MODE SEND EMAIL
    Sending from {} to {}
    Subject: {}
    Content: {}
        ",
        send_from_email, receiver_email, subject, content
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_are_separated_from_body_and_sanitized() {
        let message = format_message(
            &[("Subject", "hi\r\nBcc: x@y.test"), ("To", "a@b.test")],
            "body",
        );
        assert_eq!(
            message,
            "Subject: hiBcc: x@y.test\r\nTo: a@b.test\r\n\r\nbody"
        );
    }
}
