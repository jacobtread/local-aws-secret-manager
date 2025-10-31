use crate::{
    database::DbPool,
    handlers::{
        batch_get_secret_value::BatchGetSecretValueHandler,
        create_secret::CreateSecretHandler,
        delete_secret::DeleteSecretHandler,
        describe_secret::DescribeSecretHandler,
        error::{
            AwsErrorResponse, InternalServiceError, InvalidParameterException,
            InvalidRequestException, NotImplemented,
        },
        get_random_password::GetRandomPasswordHandler,
        get_secret_value::GetSecretValueHandler,
        list_secret_version_ids::ListSecretVersionIdsHandler,
        list_secrets::ListSecretsHandler,
        put_secret_value::PutSecretValueHandler,
        restore_secret::RestoreSecretHandler,
        tag_resource::TagResourceHandler,
        untag_resource::UntagResourceHandler,
        update_secret::UpdateSecretHandler,
        update_secret_version_stage::UpdateSecretVersionStageHandler,
    },
};
use axum::{
    Json,
    body::Body,
    http::Request,
    response::{IntoResponse, Response},
};
use futures::future::BoxFuture;
use garde::Validate;
use http_body_util::BodyExt;
use itertools::Itertools;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap, convert::Infallible, fmt::Display, str::FromStr, sync::Arc, task::Poll,
};
use thiserror::Error;
use tower::Service;
use uuid::Uuid;

pub mod batch_get_secret_value;
pub mod create_secret;
pub mod delete_secret;
pub mod describe_secret;
pub mod error;
pub mod get_random_password;
pub mod get_secret_value;
pub mod list_secret_version_ids;
pub mod list_secrets;
pub mod put_secret_value;
pub mod restore_secret;
pub mod tag_resource;
pub mod untag_resource;
pub mod update_secret;
pub mod update_secret_version_stage;

pub fn create_handlers() -> HandlerRouter {
    HandlerRouter::default()
        .add_handler("secretsmanager.CreateSecret", CreateSecretHandler)
        .add_handler("secretsmanager.DeleteSecret", DeleteSecretHandler)
        .add_handler("secretsmanager.DescribeSecret", DescribeSecretHandler)
        .add_handler("secretsmanager.GetSecretValue", GetSecretValueHandler)
        .add_handler("secretsmanager.ListSecrets", ListSecretsHandler)
        .add_handler("secretsmanager.PutSecretValue", PutSecretValueHandler)
        .add_handler("secretsmanager.UpdateSecret", UpdateSecretHandler)
        .add_handler("secretsmanager.RestoreSecret", RestoreSecretHandler)
        .add_handler("secretsmanager.TagResource", TagResourceHandler)
        .add_handler("secretsmanager.UntagResource", UntagResourceHandler)
        .add_handler("secretsmanager.GetRandomPassword", GetRandomPasswordHandler)
        .add_handler(
            "secretsmanager.ListSecretVersionIds",
            ListSecretVersionIdsHandler,
        )
        .add_handler(
            "secretsmanager.UpdateSecretVersionStage",
            UpdateSecretVersionStageHandler,
        )
        .add_handler(
            "secretsmanager.BatchGetSecretValue",
            BatchGetSecretValueHandler,
        )
}

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
        let expected = VALID_FILTER_KEYS.iter().join(", ");
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
    page_size: i64,
    /// Page index
    #[garde(skip)]
    page_index: i64,
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
struct APIErrorType {
    #[serde(rename = "ErrorCode")]
    error_code: Option<String>,

    #[serde(rename = "Message")]
    message: Option<String>,

    #[serde(rename = "SecretId")]
    secret_id: Option<String>,
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

#[derive(Default)]
pub struct HandlerRouter {
    handlers: HashMap<String, Box<dyn ErasedHandler>>,
}

impl HandlerRouter {
    pub fn add_handler<H: Handler>(mut self, target: &str, handler: H) -> Self {
        self.handlers.insert(
            target.to_string(),
            Box::new(HandlerBase { _handler: handler }),
        );
        self
    }

    pub fn get_handler(&self, target: &str) -> Option<&dyn ErasedHandler> {
        self.handlers.get(target).map(|value| value.as_ref())
    }

    pub fn into_service(self) -> HandlerRouterService {
        HandlerRouterService {
            router: Arc::new(self),
        }
    }
}

/// Service that handles routing AWS handler requests
#[derive(Clone)]
pub struct HandlerRouterService {
    router: Arc<HandlerRouter>,
}

impl Service<Request<Body>> for HandlerRouterService {
    type Response = Response;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let handlers = self.router.clone();
        Box::pin(async move {
            let (parts, body) = req.into_parts();

            let db = parts
                .extensions
                .get::<DbPool>()
                .expect("handler router service missing db pool");

            let target = match parts
                .headers
                .get("x-amz-target")
                .and_then(|v| v.to_str().ok())
            {
                Some(value) => value,
                None => {
                    return Ok(AwsErrorResponse(InvalidRequestException).into_response());
                }
            };

            let handler = handlers.get_handler(target);

            let body = match body.collect().await {
                Ok(value) => value.to_bytes(),
                Err(error) => {
                    tracing::error!(?error, "failed to collect bytes");
                    return Ok(AwsErrorResponse(InternalServiceError).into_response());
                }
            };

            Ok(match handler {
                Some(value) => value.handle(db, &body).await,
                None => AwsErrorResponse(NotImplemented).into_response(),
            })
        })
    }
}

pub trait Handler: Send + Sync + 'static {
    type Request: DeserializeOwned + Validate<Context = ()> + Send + 'static;
    type Response: Serialize + Send + 'static;

    fn handle<'d>(
        db: &'d DbPool,
        request: Self::Request,
    ) -> impl Future<Output = Result<Self::Response, Response>> + Send + 'd;
}

/// Associated type erased [Handler] that takes a generic request and provides
/// a generic response
pub trait ErasedHandler: Send + Sync + 'static {
    fn handle<'r>(&self, db: &'r DbPool, request: &'r [u8]) -> BoxFuture<'r, Response>;
}

/// Handler that takes care of the process of deserializing the request
/// type and serializing the response type to create a generic [ErasedHandler]
pub struct HandlerBase<H: Handler> {
    _handler: H,
}

impl<H: Handler> ErasedHandler for HandlerBase<H> {
    fn handle<'r>(&self, db: &'r DbPool, request: &'r [u8]) -> BoxFuture<'r, Response> {
        Box::pin(async move {
            let request: H::Request = match serde_json::from_slice(request) {
                Ok(value) => value,
                Err(error) => {
                    tracing::error!(?error, "failed to parse request");
                    return AwsErrorResponse(InvalidRequestException).into_response();
                }
            };

            if let Err(_error) = request.validate() {
                // TODO: Share the error message with the user
                return AwsErrorResponse(InvalidParameterException).into_response();
            }

            match H::handle(db, request).await {
                Ok(response) => Json(response).into_response(),
                Err(error) => error,
            }
        })
    }
}
