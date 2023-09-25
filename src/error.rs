use crate::routes::{
    newsletters::{
        auth::{BasicAuthError, CredentialsError},
        PublishNewsletterError,
    },
    subscriptions::{subscriptions_confirm::ConfirmError, SubscribeError},
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
)]
impl std::fmt::Debug for error_type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        crate::error::error_chain_fmt(self, f)
    }
}
