use argon2::{password_hash::SaltString, Algorithm, Argon2, Params, PasswordHasher, Version};
use secrecy::{ExposeSecret, Secret};

const MIN_LENGTH: usize = 12;
const MAX_LENGTH: usize = 128;

#[derive(Debug)]
pub struct Password(Secret<String>);

impl Password {
    /// Verify that a password satisfy the given password requirements.
    pub fn verify_password_requirements(
        password_candidate: Secret<String>,
    ) -> Result<Self, Vec<PasswordRequirementError>> {
        let mut errors = Vec::new();

        if password_candidate.expose_secret().len() < MIN_LENGTH {
            errors.push(PasswordRequirementError::TooShort);
        }
        if password_candidate.expose_secret().len() > MAX_LENGTH {
            errors.push(PasswordRequirementError::TooLong);
        }

        // TODO: Should check that password contains valid characters

        if errors.is_empty() {
            Ok(Password(password_candidate))
        } else {
            Err(errors)
        }
    }

    /// Compute the hash for this password.
    pub fn compute_password_hash(&self) -> Result<Secret<String>, anyhow::Error> {
        let salt = SaltString::generate(&mut rand::thread_rng());
        let password_hash = Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(15000, 2, 1, None).unwrap(),
        )
        .hash_password(self.0.expose_secret().as_bytes(), &salt)?
        .to_string();

        Ok(Secret::new(password_hash))
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PasswordRequirementError {
    #[error("Password must be at least {MIN_LENGTH} characters long")]
    TooShort,
    #[error("Password cannot be longer than {MAX_LENGTH}")]
    TooLong,
}

#[cfg(test)]
mod test {
    use super::*;
    use fake::{faker::internet::en::Password as FakePassword, Fake};
    use rstest::rstest;

    #[test]
    fn password_must_be_at_least_minimum_length_of_characters() {
        let password_candidate: Secret<String> = Secret::new(FakePassword(0..MIN_LENGTH).fake());

        let password = Password::verify_password_requirements(password_candidate);
        assert!(password
            .unwrap_err()
            .contains(&PasswordRequirementError::TooShort));
    }

    #[test]
    fn password_must_not_be_longer_than_the_maximum_length() {
        let password_candidate: Secret<String> =
            Secret::new(FakePassword(MAX_LENGTH + 1..1024).fake());

        let password = Password::verify_password_requirements(password_candidate);
        assert!(password
            .unwrap_err()
            .contains(&PasswordRequirementError::TooLong));
    }

    #[rstest]
    #[case("abcdefghijkl")]
    fn returns_valid_password(#[case] password_candidate: Secret<String>) {
        assert!(Password::verify_password_requirements(password_candidate).is_ok());
    }
}
