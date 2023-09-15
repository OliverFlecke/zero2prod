//! This email client is currently just a mock and not really doing anything.
//! The API used by zero2prod is not available anymore for just everyone for free,
//! and did not finding an free easy alternative.

use crate::{configuration::EmailClientSettings, domain::SubscriberEmail};
use reqwest::{Client, Url};
use secrecy::{ExposeSecret, Secret};

#[derive(Debug)]
pub struct EmailClient {
    base_url: Url,
    sender: SubscriberEmail,
    http_client: Client,
    authorization_token: Secret<String>,
}

impl EmailClient {
    /// Create a new email client.
    pub fn new(
        base_url: Url,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
    ) -> Self {
        Self {
            base_url,
            sender,
            http_client: Client::new(),
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> Result<(), reqwest::Error> {
        let url = self
            .base_url
            .join("email")
            .expect("url to always be valid at this point");
        let request_body = SendEmailRequest {
            from: self.sender.as_ref(),
            to: recipient.as_ref(),
            subject,
            text_body,
            html_body,
        };

        self.http_client
            .post(url)
            .header(
                "X-Postmark-Server-Token",
                self.authorization_token.expose_secret(),
            )
            .json(&request_body)
            .send()
            .await?;

        Ok(())
    }
}

impl TryFrom<&EmailClientSettings> for EmailClient {
    type Error = String;

    fn try_from(config: &EmailClientSettings) -> Result<Self, Self::Error> {
        Ok(Self::new(
            config.base_url().map_err(|e| {
                tracing::error!("Unable to parse email client's base url: {e}");
                "Email base url is invalid".to_string()
            })?,
            config.sender()?,
            config.authorization_token().clone(),
        ))
    }
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct SendEmailRequest<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    text_body: &'a str,
    html_body: &'a str,
}

#[cfg(test)]
mod tests {
    use crate::{domain::SubscriberEmail, email_client::EmailClient};
    use fake::{
        faker::{
            internet::en::SafeEmail,
            lorem::en::{Paragraph, Sentence},
        },
        Fake, Faker,
    };
    use http::StatusCode;
    use reqwest::Url;
    use secrecy::Secret;
    use wiremock::{
        matchers::{header, header_exists, method, path},
        Mock, MockServer, Request, ResponseTemplate,
    };

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> = serde_json::from_slice(&request.body);

            if let Ok(body) = result {
                body.get("From").is_some()
                    && body.get("To").is_some()
                    && body.get("Subject").is_some()
                    && body.get("HtmlBody").is_some()
                    && body.get("TextBody").is_some()
            } else {
                false
            }
        }
    }

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        // Arrange
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(
            Url::parse(&mock_server.uri()).unwrap(),
            sender,
            Secret::new(Faker.fake()),
        );

        Mock::given(header_exists("X-Postmark-Server-Token"))
            .and(header("Content-Type", "application/json"))
            .and(path("/email"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(StatusCode::OK))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        // Act
        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;

        // Assert
    }
}
