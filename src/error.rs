use crate::{
    authorization::{BasicAuthError, CredentialsError},
    metrics::MetricsError,
    require_login::AuthorizedUserError,
    routes::{
        admin::{newsletters::PublishNewsletterError, password::ChangePasswordError},
        login::post::LoginError,
        subscriptions::{subscriptions_confirm::ConfirmError, StoreTokenError, SubscribeError},
    },
    state::session::TypedSessionError,
};
use duplicate::duplicate_item;

/// Write a formatted version of the error and its inner source.
pub fn error_chain_fmt(
    e: &impl std::error::Error,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    writeln!(f, "{e}\n")?;
    let mut current = e.source();
    while let Some(cause) = current {
        writeln!(f, "Caused by:\n\t{cause}")?;
        current = cause.source();
    }

    Ok(())
}

#[duplicate_item(
    error_type;
    [ BasicAuthError ];
    [ PublishNewsletterError ];
    [ SubscribeError ];
    [ ConfirmError ];
    [ CredentialsError ];
    [ LoginError ];
    [ TypedSessionError ];
    [ ChangePasswordError ];
    [ AuthorizedUserError ];
    [ StoreTokenError ];
    [ MetricsError ];
)]
impl std::fmt::Debug for error_type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::error::error_chain_fmt(self, f)
    }
}
