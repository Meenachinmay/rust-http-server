// email.rs

use sendgrid::v3::{Content, Email, Message, Personalization, Sender};
use reqwest::Client;
use std::env;

// We create a dedicated error type for email-related operations
#[derive(Debug)]
pub enum EmailError {
    EnvVarMissing(String),
    SendGridError(String),
    ConfigError(String),
}

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmailError::EnvVarMissing(var) => write!(f, "Missing environment variable: {}", var),
            EmailError::SendGridError(err) => write!(f, "SendGrid error: {}", err),
            EmailError::ConfigError(err) => write!(f, "Configuration error: {}", err),
        }
    }
}

impl std::error::Error for EmailError {}

pub async fn send_verification_email(to_email: &str, token: &str) -> Result<(), EmailError> {
    // Retrieve necessary environment variables
    let sendgrid_api_key = env::var("SENDGRID_API_KEY")
        .map_err(|_| EmailError::EnvVarMissing("SENDGRID_API_KEY".to_string()))?;

    let frontend_url = env::var("FRONTEND_URL")
        .map_err(|_| EmailError::EnvVarMissing("FRONTEND_URL".to_string()))?;

    let sender_email = env::var("SENDER_EMAIL")
        .map_err(|_| EmailError::EnvVarMissing("SENDER_EMAIL".to_string()))?;

    // Create a default HTTP client
    let client = Client::new();

    // Create the verification URL with the token
    let verification_url = format!("{}/verify?token={}", frontend_url, token);

    // Create HTML content for the email
    let html_content = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <body style="font-family: Arial, sans-serif; line-height: 1.6; color: #333;">
            <div style="max-width: 600px; margin: 0 auto; padding: 20px;">
                <h2>Verify Your Email Address</h2>
                <p>Thank you for signing up! Please click the button below to verify your email and set your password:</p>
                <div style="text-align: center; margin: 30px 0;">
                    <a href="{}"
                       style="background-color: #4CAF50;
                              color: white;
                              padding: 12px 24px;
                              text-decoration: none;
                              border-radius: 4px;
                              display: inline-block;">
                        Verify Email
                    </a>
                </div>
                <p>If the button doesn't work, you can copy and paste this link into your browser:</p>
                <p style="word-break: break-all;">{}</p>
                <p>This link will expire in 24 hours.</p>
                <p>If you didn't request this verification, please ignore this email.</p>
            </div>
        </body>
        </html>
        "#,
        verification_url,
        verification_url
    );

    // Create the SendGrid message
    let personalization = Personalization::new(Email::new(to_email));

    let message = Message::new(Email::new(&sender_email))
        .set_subject("Verify Your Email Address")
        .add_content(
            Content::new()
                .set_content_type("text/html")
                .set_value(&html_content)
        )
        .add_personalization(personalization);

    // Create the SendGrid sender
    let sender = Sender::new(sendgrid_api_key, Some(client));

    // Send the email
    sender
        .send(&message)
        .await
        .map_err(|e| EmailError::SendGridError(e.to_string()))?;

    Ok(())
}