use unicode_segmentation::UnicodeSegmentation;

/// Struct to hold the validated name of a subscriber.
/// The only way to create a `SubscriberName` is through the validated methods
/// in this module, which means consumers of this type is always guaranteed that
/// it will contain a valid subscriber name.
#[derive(Debug)]
pub struct SubscriberName(String);

impl SubscriberName {
    /// Returns an instance of `SubscriberName` if the input satisfies all
    /// out validation constrations on subscriber names.
    /// It panics otherwise.
    pub fn parse(s: String) -> Result<Self, String> {
        let is_empty_or_whitespace = s.trim().is_empty();

        // Using graphemes as some characters are preceived as a single character
        // but is composed of two characters.
        let is_too_long = s.graphemes(true).count() > 256;

        let forbidden_characters = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let contains_forbidden_characters = s.chars().any(|g| forbidden_characters.contains(&g));

        if is_empty_or_whitespace || is_too_long || contains_forbidden_characters {
            Err(format!("{s} is not a valid subscriber name."))
        } else {
            Ok(Self(s))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::SubscriberName;
    use claims::{assert_err, assert_ok};
    use rstest::*;

    #[rstest]
    #[case("")]
    #[case("/")]
    #[case("(")]
    #[case(")")]
    #[case("\"")]
    #[case("<")]
    #[case(">")]
    #[case("\\")]
    #[case("{")]
    #[case("}")]
    fn invalid_characters_are_rejected(#[case] input: String) {
        assert_err!(SubscriberName::parse(input));
    }

    #[rstest]
    #[case("")]
    #[case(" ")]
    #[case("\n")]
    #[case("\t")]
    fn whitespace_only_names_are_rejected(#[case] input: String) {
        assert_err!(SubscriberName::parse(input));
    }

    #[test]
    fn a_256_grapheme_long_name_is_valid() {
        let name = "å".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_257_grapheme_long_name_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn a_valid_name_is_parsed_successfully() {
        let name = "Ursula Le Guin".to_string();
        assert_ok!(SubscriberName::parse(name));
    }
}
