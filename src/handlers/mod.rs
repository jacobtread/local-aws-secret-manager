use crate::{
    database::DbPool,
    handlers::{
        batch_get_secret_value::BatchGetSecretValueHandler,
        create_secret::CreateSecretHandler,
        delete_secret::DeleteSecretHandler,
        describe_secret::DescribeSecretHandler,
        error::{AwsErrorResponse, InternalServiceError, InvalidRequestException, NotImplemented},
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
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap, convert::Infallible, fmt::Display, str::FromStr, sync::Arc, task::Poll,
};
use thiserror::Error;
use tower::Service;

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

#[derive(Deserialize, Serialize)]
struct Tag {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Value")]
    value: String,
}

#[derive(Deserialize, Serialize)]
struct Filter {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Values")]
    values: Vec<String>,
}

pub struct PaginationToken {
    /// Size of each page
    page_size: i64,
    /// Page index
    page_index: i64,
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
    type Request: DeserializeOwned + Send + 'static;
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

            match H::handle(db, request).await {
                Ok(response) => Json(response).into_response(),
                Err(error) => error,
            }
        })
    }
}
