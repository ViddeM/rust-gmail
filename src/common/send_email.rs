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

impl GoogleSendEmailRequest {
    pub fn new(from: &str, to: &str, subject: &str, content: &str) -> Self {
        let raw_message = base64::encode(format!(
            "From: {}\r\nTo: {}\r\nSubject: {}\r\n\r\n{}\r\n",
            from, to, subject, content
        ));
        Self { raw: raw_message }
    }

    pub fn from_raw(message: &str) -> Self {
        Self {
            raw: base64::encode_config(message, base64::URL_SAFE),
        }
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
