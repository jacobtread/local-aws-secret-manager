use garde::Validate;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, str::FromStr};
use thiserror::Error;
use uuid::Uuid;

use crate::utils::string::join_iter_string;

#[derive(Debug, Deserialize, Validate)]
#[garde(transparent)]
pub struct SecretName(
    #[garde(length(min = 1, max = 512))]
    #[garde(custom(is_valid_secret_name))]
    pub String,
);

impl Display for SecretName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Checks if the provided value is a valid filter key
fn is_valid_secret_name(value: &str, _context: &()) -> garde::Result {
    const ALLOWED_SPECIAL_CHARACTERS: &str = "/_+=.@-";

    if !value
        .chars()
        .all(|char| char.is_ascii_alphanumeric() || ALLOWED_SPECIAL_CHARACTERS.contains(char))
    {
        return Err(garde::Error::new(
            "secret name contains disallowed characters",
        ));
    }

    Ok(())
}

#[derive(Deserialize, Validate)]
#[garde(transparent)]
pub struct ClientRequestToken(#[garde(length(min = 32, max = 64))] pub String);

impl Default for ClientRequestToken {
    fn default() -> Self {
        Self(Uuid::new_v4().to_string())
    }
}

#[derive(Debug, Deserialize, Validate)]
#[garde(transparent)]
pub struct SecretId(#[garde(length(min = 1, max = 2048))] pub String);

impl Display for SecretId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Deserialize, Validate)]
#[garde(transparent)]
pub struct VersionId(#[garde(length(min = 32, max = 64))] pub String);

impl VersionId {
    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Deserialize, Validate)]
#[garde(transparent)]
pub struct SecretString(#[garde(length(min = 1, max = 65536))] pub String);

impl SecretString {
    pub fn into_inner(self) -> String {
        self.0
    }
}

/// TODO: Check if the length constraint here should be on the base64 value
/// or the decoded blob itself
#[derive(Deserialize, Validate)]
#[garde(transparent)]
pub struct SecretBinary(#[garde(length(min = 1, max = 65536))] pub String);

impl SecretBinary {
    pub fn into_inner(self) -> String {
        self.0
    }
}

#[derive(Deserialize, Serialize, Validate)]
pub struct Tag {
    #[serde(rename = "Key")]
    #[garde(length(min = 1, max = 128))]
    pub key: String,

    #[serde(rename = "Value")]
    #[garde(length(min = 1, max = 256))]
    pub value: String,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct Filter {
    #[serde(rename = "Key")]
    #[garde(custom(is_valid_filter_key))]
    pub key: String,

    #[serde(rename = "Values")]
    #[garde(
        length(min = 1, max = 10),
        inner(custom(is_valid_filter_value)),
        inner(length(min = 1, max = 512))
    )]
    pub values: Vec<String>,
}

const VALID_FILTER_KEYS: [&str; 7] = [
    "description",
    "name",
    "tag-key",
    "tag-value",
    "primary-region",
    "owning-service",
    "all",
];

/// Checks if the provided value is a valid filter key
fn is_valid_filter_key(value: &str, _context: &()) -> garde::Result {
    if !VALID_FILTER_KEYS.contains(&value) {
        let expected = join_iter_string(VALID_FILTER_KEYS.iter(), ", ");
        return Err(garde::Error::new(format!(
            "unknown filter key expected one of: {expected}"
        )));
    }

    Ok(())
}

/// Checks if the provided value is a valid filter value
fn is_valid_filter_value(value: &str, _context: &()) -> garde::Result {
    const ALLOWED_SPECIAL_CHARACTERS: &str = " :_@/+=.-!";

    let mut chars = value.chars();

    // Check optional '!' at the start
    if let Some('!') = chars.clone().next() {
        chars.next(); // skip the '!'
    }

    // Check remaining characters
    for char in chars {
        if !char.is_ascii_alphanumeric() && !ALLOWED_SPECIAL_CHARACTERS.contains(char) {
            return Err(garde::Error::new(
                "filter value contains disallowed characters",
            ));
        }
    }

    Ok(())
}

#[derive(Validate)]
pub struct PaginationToken {
    /// Size of each page
    #[garde(skip)]
    pub page_size: i64,
    /// Page index
    #[garde(skip)]
    pub page_index: i64,
}

impl PaginationToken {
    /// Update the page size
    pub fn page_size(mut self, page_size: impl Into<i64>) -> Self {
        self.page_size = page_size.into();
        self
    }

    // Compute the limit and offset to use for database queries
    pub fn as_query_parts(&self) -> Option<(i64, i64)> {
        let limit = self.page_size;
        let offset = self.page_size.checked_mul(self.page_index)?;
        Some((limit, offset))
    }

    /// Get the next page if one fits within the bounds of count
    pub fn get_next_page(&self, count: i64) -> Option<PaginationToken> {
        let next_page = self.page_index.checked_add(1)?;
        let next_page_offset = next_page.checked_mul(self.page_size)?;

        if count <= next_page_offset {
            return None;
        }

        Some(PaginationToken {
            page_size: self.page_size,
            page_index: next_page,
        })
    }
}

impl<'de> Deserialize<'de> for PaginationToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PaginationToken::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Serialize)]
pub struct APIErrorType {
    #[serde(rename = "ErrorCode")]
    pub error_code: Option<String>,

    #[serde(rename = "Message")]
    pub message: Option<String>,

    #[serde(rename = "SecretId")]
    pub secret_id: Option<String>,
}

impl Display for PaginationToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.page_size, self.page_index)
    }
}

#[derive(Debug, Error)]
#[error("invalid pagination token")]
pub struct InvalidPaginationToken;

impl FromStr for PaginationToken {
    type Err = InvalidPaginationToken;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (page_size, page) = s.split_once(":").ok_or(InvalidPaginationToken)?;
        let page_size = page_size.parse().map_err(|_| InvalidPaginationToken)?;
        let page = page.parse().map_err(|_| InvalidPaginationToken)?;

        Ok(PaginationToken {
            page_size,
            page_index: page,
        })
    }
}
