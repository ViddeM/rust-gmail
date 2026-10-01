use crate::{
    common::{
        calendar::{mock_print_calendar_event, CalendarEvent},
        send_email::{
            GoogleSendEmailRequest, GoogleSendEmailResponse, SEND_EMAIL_ENDPOINT,
            SEND_EMAIL_QUERY_PARAMETERS,
        },
    },
    error::{GoogleApiError, Result},
};

pub async fn send_calendar_event(
    receiver_email: &str,
    subject: &str,
    content: &str,
    event: &CalendarEvent,
    token: &str,
    send_from_email: &str,
    mock_mode: bool,
) -> Result<()> {
    if mock_mode {
        mock_print_calendar_event(receiver_email, subject, content, send_from_email, event);
        return Ok(());
    }

    let request = GoogleSendEmailRequest::new_calendar_event(
        send_from_email,
        receiver_email,
        subject,
        content,
        event,
    );

    let client = reqwest::Client::new();
    let response_text = client
        .post(SEND_EMAIL_ENDPOINT)
        .query(&SEND_EMAIL_QUERY_PARAMETERS)
        .header(reqwest::header::AUTHORIZATION, format!("Bearer {}", token))
        .json(&request)
        .send()
        .await?
        .text()
        .await?;

    let _response: GoogleSendEmailResponse = serde_json::from_str(&response_text)
        .map_err(|_| GoogleApiError::EmailSendError(response_text))?;

    Ok(())
}
