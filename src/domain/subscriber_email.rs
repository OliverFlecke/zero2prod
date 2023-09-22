use std::fmt::Display;

use validator::validate_email;

/// Represents a valid email to a subscriber.
#[derive(Debug)]
pub struct SubscriberEmail(String);

impl SubscriberEmail {
    pub fn parse(s: String) -> Result<Self, String> {
        if validate_email(&s) {
            Ok(Self(s))
        } else {
            Err(format!("{s} is not a valid subscriber email."))
        }
    }
}

impl Display for SubscriberEmail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberEmail;
    use claims::assert_err;
    use fake::{faker::internet::en::SafeEmail, Fake};
    use proptest::prelude::*;
    use rstest::*;

    #[rstest]
    #[case("")]
    fn empty_string_is_rejected(#[case] email: String) {
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_at_symbol_is_rejected() {
        let email = "ursuladomain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn email_missing_subject_is_rejected() {
        let email = "@domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[derive(Debug, Clone)]
    struct ValidEmailFixture(pub String);

    fn email() -> impl Strategy<Value = ValidEmailFixture> {
        any::<u32>().prop_map(|_| ValidEmailFixture(SafeEmail().fake()))
    }

    proptest! {
        #[test]
        fn valid_email_are_parsed_successfully(valid_email in email()) {
            claims::assert_ok!(SubscriberEmail::parse(valid_email.0));
        }
    }
}
