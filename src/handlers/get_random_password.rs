use axum::response::{IntoResponse, Response};
use rand::seq::{IndexedRandom, SliceRandom};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    database::DbPool,
    handlers::{
        Handler,
        error::{AwsErrorResponse, InvalidRequestException},
    },
};

// https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetRandomPassword.html
pub struct GetRandomPasswordHandler;

#[derive(Deserialize)]
pub struct GetRandomPasswordRequest {
    #[serde(rename = "ExcludeCharacters")]
    exclude_characters: Option<String>,
    #[serde(rename = "ExcludeLowercase")]
    exclude_lowercase: Option<bool>,
    #[serde(rename = "ExcludeNumbers")]
    exclude_numbers: Option<bool>,
    #[serde(rename = "ExcludePunctuation")]
    exclude_punctuation: Option<bool>,
    #[serde(rename = "ExcludeUppercase")]
    exclude_uppercase: Option<bool>,
    #[serde(rename = "IncludeSpace")]
    include_space: Option<bool>,
    #[serde(rename = "PasswordLength")]
    password_length: Option<i64>,
    #[serde(rename = "RequireEachIncludedType")]
    require_each_included_type: Option<bool>,
}

#[derive(Serialize)]
pub struct GetRandomPasswordResponse {
    #[serde(rename = "RandomPassword")]
    random_password: String,
}

impl Handler for GetRandomPasswordHandler {
    type Request = GetRandomPasswordRequest;
    type Response = GetRandomPasswordResponse;

    async fn handle(_db: &DbPool, request: Self::Request) -> Result<Self::Response, Response> {
        let exclude_characters = request.exclude_characters.unwrap_or_default();
        let exclude_lowercase = request.exclude_lowercase.unwrap_or_default();
        let exclude_numbers = request.exclude_numbers.unwrap_or_default();
        let exclude_punctuation = request.exclude_punctuation.unwrap_or_default();
        let exclude_uppercase = request.exclude_uppercase.unwrap_or_default();
        let include_space = request.include_space.unwrap_or_default();
        let password_length = request.password_length.unwrap_or(32);
        let require_each_included_type = request.require_each_included_type.unwrap_or_default();

        let random_password = match get_random_password(PasswordOptions {
            exclude_characters,
            exclude_lowercase,
            exclude_numbers,
            exclude_punctuation,
            exclude_uppercase,
            include_space,
            password_length: password_length as usize,
            require_each_included_type,
        }) {
            Ok(value) => value,
            Err(_error) => {
                return Err(AwsErrorResponse(InvalidRequestException).into_response());
            }
        };

        Ok(GetRandomPasswordResponse { random_password })
    }
}

const LOWERCASE: &str = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const NUMBERS: &str = "0123456789";
const PUNCTUATION: &str = "!\"#$%&'()*+,-./:;<=>?@[\\]^_`{|}~";

#[derive(Debug)]
struct PasswordOptions {
    pub exclude_characters: String,
    pub exclude_lowercase: bool,
    pub exclude_numbers: bool,
    pub exclude_punctuation: bool,
    pub exclude_uppercase: bool,
    pub include_space: bool,
    pub password_length: usize,
    pub require_each_included_type: bool,
}

#[derive(Debug, Error)]
enum RandomPasswordError {
    /// All of the possible characters are excluded
    #[error("empty char set")]
    EmptyCharSet,
    /// One of the required type sets has been completely excluded by
    /// the user provided exclude_characters
    #[error("empty type set")]
    EmptyTypeSet,

    /// The `require_each_included_type` option was provided but the
    /// requested length is too short to fit one of each
    #[error("invalid length requirement")]
    InvalidLength,
}

/// Generate a random password from the provided options
fn get_random_password(opts: PasswordOptions) -> Result<String, RandomPasswordError> {
    // Take the input charset string and provide a collection of chars
    // that aren't present in the excluded list
    fn filter_allowed(set: &str, excluded: &str) -> Vec<char> {
        set.chars()
            .filter(|item| !excluded.contains(*item))
            .collect()
    }

    let lowercase: Option<Vec<char>> = if opts.exclude_lowercase {
        None
    } else {
        Some(filter_allowed(LOWERCASE, &opts.exclude_characters))
    };

    let uppercase: Option<Vec<char>> = if opts.exclude_uppercase {
        None
    } else {
        Some(filter_allowed(UPPERCASE, &opts.exclude_characters))
    };

    let numbers: Option<Vec<char>> = if opts.exclude_numbers {
        None
    } else {
        Some(filter_allowed(NUMBERS, &opts.exclude_characters))
    };

    let punctuation: Option<Vec<char>> = if opts.exclude_punctuation {
        None
    } else {
        Some(filter_allowed(PUNCTUATION, &opts.exclude_characters))
    };

    // Collect character sets by allowed type
    let type_sets: Vec<Vec<char>> = lowercase
        .into_iter()
        .chain(uppercase)
        .chain(numbers)
        .chain(punctuation)
        .collect();

    // Collect all available characters
    let mut allowed: Vec<char> = type_sets.iter().flatten().copied().collect();

    // Add a space to the available characters if its nto excluded
    if opts.include_space && !opts.exclude_characters.contains(' ') {
        allowed.push(' ');
    }

    if allowed.is_empty() {
        return Err(RandomPasswordError::EmptyCharSet);
    }

    let length = opts.password_length;

    let mut rng = rand::rng();

    if opts.require_each_included_type {
        let mut password_chars: Vec<char> = Vec::with_capacity(length);

        if length < type_sets.len() {
            return Err(RandomPasswordError::InvalidLength);
        }

        // Include one random item from each type set
        for set in type_sets {
            let char = set
                .choose(&mut rng)
                .ok_or(RandomPasswordError::EmptyTypeSet)?;
            password_chars.push(*char);
        }

        // Fill the rest from allowed characters
        while password_chars.len() < length {
            let char = allowed
                .choose(&mut rng)
                .ok_or(RandomPasswordError::EmptyCharSet)?;
            password_chars.push(*char);
        }

        // Shuffle so the required characters are not all at the front
        password_chars.shuffle(&mut rng);

        Ok(password_chars.into_iter().collect())
    } else {
        let mut password: String = String::with_capacity(length);

        // Fill from allowed characters
        for _ in 0..length {
            let char = allowed
                .choose(&mut rng)
                .ok_or(RandomPasswordError::EmptyCharSet)?;

            password.push(*char);
        }

        Ok(password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_options() {
        let options = PasswordOptions {
            exclude_characters: "".to_string(),
            exclude_lowercase: false,
            exclude_numbers: false,
            exclude_punctuation: false,
            exclude_uppercase: false,
            include_space: false,
            password_length: 32,
            require_each_included_type: false,
        };
        let value = get_random_password(options).unwrap();

        let mut allowed = String::new();
        allowed.push_str(LOWERCASE);
        allowed.push_str(UPPERCASE);
        allowed.push_str(NUMBERS);
        allowed.push_str(PUNCTUATION);

        // Ensures only the requested characters are included
        assert!(value.chars().all(|c| allowed.contains(c)));

        assert_eq!(value.len(), 32);
    }

    #[test]
    fn test_length_requirement() {
        let options = PasswordOptions {
            exclude_characters: "".to_string(),
            exclude_lowercase: false,
            exclude_numbers: false,
            exclude_punctuation: false,
            exclude_uppercase: false,
            include_space: false,
            password_length: 48,
            require_each_included_type: false,
        };
        let value = get_random_password(options).unwrap();

        let mut allowed = String::new();
        allowed.push_str(LOWERCASE);
        allowed.push_str(UPPERCASE);
        allowed.push_str(NUMBERS);
        allowed.push_str(PUNCTUATION);

        // Ensures only the requested characters are included
        assert!(value.chars().all(|c| allowed.contains(c)));
        assert_eq!(value.len(), 48);
    }

    #[test]
    fn test_require_each_included_type() {
        let options = PasswordOptions {
            exclude_characters: "".to_string(),
            exclude_lowercase: false,
            exclude_numbers: false,
            exclude_punctuation: false,
            exclude_uppercase: false,
            include_space: false,
            password_length: 48,
            require_each_included_type: false,
        };
        let value = get_random_password(options).unwrap();

        // Must included one of each of the types
        assert!(value.chars().any(|c| LOWERCASE.contains(c)));
        assert!(value.chars().any(|c| UPPERCASE.contains(c)));
        assert!(value.chars().any(|c| NUMBERS.contains(c)));
        assert!(value.chars().any(|c| PUNCTUATION.contains(c)));

        assert_eq!(value.len(), 48);
    }

    #[test]
    fn test_require_each_included_type_exclude_lowercase() {
        let options = PasswordOptions {
            exclude_characters: "".to_string(),
            exclude_lowercase: true,
            exclude_numbers: false,
            exclude_punctuation: false,
            exclude_uppercase: false,
            include_space: false,
            password_length: 48,
            require_each_included_type: false,
        };
        let value = get_random_password(options).unwrap();

        // Must included one of each of the types
        assert!(value.chars().any(|c| UPPERCASE.contains(c)));
        assert!(value.chars().any(|c| NUMBERS.contains(c)));
        assert!(value.chars().any(|c| PUNCTUATION.contains(c)));

        assert_eq!(value.len(), 48);
    }

    #[test]
    fn test_exclude_characters() {
        let excluded = "az1";

        let options = PasswordOptions {
            exclude_characters: excluded.to_string(),
            exclude_lowercase: false,
            exclude_numbers: false,
            exclude_punctuation: false,
            exclude_uppercase: false,
            include_space: false,
            password_length: 48,
            require_each_included_type: false,
        };
        let value = get_random_password(options).unwrap();

        // Ensures none of the excluded characters are included
        assert!(value.chars().all(|c| !excluded.contains(c)));

        assert_eq!(value.len(), 48);
    }

    #[test]
    fn test_exclude_lowercase() {
        let options = PasswordOptions {
            exclude_characters: "".to_string(),
            exclude_lowercase: true,
            exclude_numbers: false,
            exclude_punctuation: false,
            exclude_uppercase: false,
            include_space: false,
            password_length: 48,
            require_each_included_type: false,
        };
        let value = get_random_password(options).unwrap();

        let mut allowed = String::new();
        allowed.push_str(UPPERCASE);
        allowed.push_str(NUMBERS);
        allowed.push_str(PUNCTUATION);

        // Ensures only the requested characters are included
        assert!(value.chars().all(|c| allowed.contains(c)));
        assert_eq!(value.len(), 48);
    }

    #[test]
    fn test_exclude_uppercase() {
        let options = PasswordOptions {
            exclude_characters: "".to_string(),
            exclude_lowercase: false,
            exclude_numbers: false,
            exclude_punctuation: false,
            exclude_uppercase: true,
            include_space: false,
            password_length: 48,
            require_each_included_type: false,
        };
        let value = get_random_password(options).unwrap();

        let mut allowed = String::new();
        allowed.push_str(LOWERCASE);
        allowed.push_str(NUMBERS);
        allowed.push_str(PUNCTUATION);

        // Ensures only the requested characters are included
        assert!(value.chars().all(|c| allowed.contains(c)));
        assert_eq!(value.len(), 48);
    }

    #[test]
    fn test_exclude_numbers() {
        let options = PasswordOptions {
            exclude_characters: "".to_string(),
            exclude_lowercase: false,
            exclude_numbers: true,
            exclude_punctuation: false,
            exclude_uppercase: false,
            include_space: false,
            password_length: 48,
            require_each_included_type: false,
        };
        let value = get_random_password(options).unwrap();

        let mut allowed = String::new();
        allowed.push_str(LOWERCASE);
        allowed.push_str(UPPERCASE);
        allowed.push_str(PUNCTUATION);

        // Ensures only the requested characters are included
        assert!(value.chars().all(|c| allowed.contains(c)));
        assert_eq!(value.len(), 48);
    }

    #[test]
    fn test_exclude_punctuation() {
        let options = PasswordOptions {
            exclude_characters: "".to_string(),
            exclude_lowercase: false,
            exclude_numbers: false,
            exclude_punctuation: true,
            exclude_uppercase: false,
            include_space: false,
            password_length: 48,
            require_each_included_type: false,
        };
        let value = get_random_password(options).unwrap();

        let mut allowed = String::new();
        allowed.push_str(LOWERCASE);
        allowed.push_str(UPPERCASE);
        allowed.push_str(NUMBERS);

        // Ensures only the requested characters are included
        assert!(value.chars().all(|c| allowed.contains(c)));
        assert_eq!(value.len(), 48);
    }

    #[test]
    fn test_exclude_everything_error() {
        let options = PasswordOptions {
            exclude_characters: "".to_string(),
            exclude_lowercase: true,
            exclude_numbers: true,
            exclude_punctuation: true,
            exclude_uppercase: true,
            include_space: false,
            password_length: 48,
            require_each_included_type: false,
        };
        let value = get_random_password(options).unwrap_err();
        assert!(matches!(value, RandomPasswordError::EmptyCharSet));
    }

    #[test]
    fn test_too_short_for_everything() {
        let options = PasswordOptions {
            exclude_characters: "".to_string(),
            exclude_lowercase: false,
            exclude_numbers: false,
            exclude_punctuation: false,
            exclude_uppercase: false,
            include_space: false,
            password_length: 1,
            require_each_included_type: true,
        };
        let value = get_random_password(options).unwrap_err();
        assert!(matches!(value, RandomPasswordError::InvalidLength));
    }

    #[test]
    fn test_required_type_set_is_excluded() {
        let options = PasswordOptions {
            exclude_characters: LOWERCASE.to_string(),
            exclude_lowercase: false,
            exclude_numbers: false,
            exclude_punctuation: false,
            exclude_uppercase: false,
            include_space: false,
            password_length: 32,
            require_each_included_type: true,
        };
        let value = get_random_password(options).unwrap_err();
        assert!(matches!(value, RandomPasswordError::EmptyTypeSet));
    }
}
