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
/// CR/LF in header names and values are stripped to prevent header injection, and non-ASCII
/// header values are encoded as RFC 2047 encoded-words.
pub fn format_message(headers: &Headers, body: &str) -> String {
    let mut message = String::new();
    for (name, value) in headers {
        let clean = |s: &str| s.replace(['\r', '\n'], "");
        let (name, value) = (clean(name), clean(value));
        let value = if is_address_header(&name) {
            encode_address(&value)
        } else {
            encode_header_value(&value)
        };
        message.push_str(&format!("{}: {}\r\n", name, value));
    }
    message.push_str("\r\n");
    message.push_str(body);
    message
}

fn is_address_header(name: &str) -> bool {
    ["from", "to", "cc", "bcc", "reply-to"]
        .iter()
        .any(|h| name.eq_ignore_ascii_case(h))
}

/// Encode only the display name of `Name <addr>`, leaving the address untouched.
fn encode_address(value: &str) -> String {
    match value.rfind('<') {
        Some(idx) if value.ends_with('>') => {
            let name = value[..idx].trim();
            format!("{} {}", encode_header_value(name), &value[idx..])
        }
        _ => value.to_string(),
    }
}

/// Encode `value` as RFC 2047 encoded-words (UTF-8, base64) if it contains non-ASCII characters.
fn encode_header_value(value: &str) -> String {
    if value.is_ascii() {
        return value.to_string();
    }

    // Each encoded-word may be at most 75 characters: 12 of overhead leaves 63 base64
    // characters, i.e. 45 bytes of input. Split on char boundaries.
    const MAX_BYTES: usize = 45;
    let mut words = Vec::new();
    let mut chunk = String::new();
    for c in value.chars() {
        if chunk.len() + c.len_utf8() > MAX_BYTES {
            words.push(std::mem::take(&mut chunk));
        }
        chunk.push(c);
    }
    if !chunk.is_empty() {
        words.push(chunk);
    }

    words
        .iter()
        .map(|w| format!("=?UTF-8?B?{}?=", base64::encode(w)))
        .collect::<Vec<_>>()
        .join("\r\n ")
}

/// Base64 encode `data`, wrapped at 76 characters with CRLF as MIME requires.
pub fn wrapped_base64(data: &[u8]) -> String {
    base64::encode(data)
        .as_bytes()
        .chunks(76)
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect::<Vec<_>>()
        .join("\r\n")
}

impl GoogleSendEmailRequest {
    pub fn new(from: &str, to: &str, subject: &str, content: &str) -> Self {
        Self::from_headers(
            &[
                ("From", from),
                ("To", to),
                ("Subject", subject),
                ("MIME-Version", "1.0"),
                ("Content-Type", "text/plain; charset=\"UTF-8\""),
                ("Content-Transfer-Encoding", "base64"),
            ],
            &wrapped_base64(content.as_bytes()),
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
    fn non_ascii_headers_are_encoded() {
        let m = format_message(
            &[("Subject", "Möte åäö"), ("To", "Åsa Öberg <a@b.test>")],
            "",
        );
        assert!(m.contains("Subject: =?UTF-8?B?TcO2dGUgw6XDpMO2?=\r\n"));
        assert!(m.contains("To: =?UTF-8?B?w4VzYSDDlmJlcmc=?= <a@b.test>\r\n"));
        assert!(m.is_ascii());
    }

    #[test]
    fn long_non_ascii_subject_is_split_into_valid_words() {
        let m = encode_header_value(&"å".repeat(100));
        for word in m.split("\r\n ") {
            assert!(word.len() <= 75);
        }
    }

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
