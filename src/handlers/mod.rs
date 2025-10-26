use crate::{
    database::DbPool,
    handlers::{
        create_secret::CreateSecretHandler,
        delete_secret::DeleteSecretHandler,
        describe_secret::DescribeSecretHandler,
        error::{AwsErrorResponse, InternalServiceError, InvalidRequestException, NotImplemented},
        get_secret_value::GetSecretValueHandler,
        list_secrets::ListSecretsHandler,
        put_secret_value::PutSecretValueHandler,
        restore_secret::RestoreSecretHandler,
        tag_resource::TagResourceHandler,
        untag_resource::UntagResourceHandler,
        update_secret::UpdateSecretHandler,
    },
};
use axum::{
    Json,
    body::Body,
    http::Request,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::HashMap, convert::Infallible, pin::Pin, sync::Arc, task::Poll};
use tower::Service;

pub mod create_secret;
pub mod delete_secret;
pub mod describe_secret;
pub mod error;
pub mod get_secret_value;
pub mod list_secrets;
pub mod put_secret_value;
pub mod restore_secret;
pub mod tag_resource;
pub mod untag_resource;
pub mod update_secret;

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
}

#[derive(Deserialize, Serialize)]
struct Tag {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Value")]
    value: String,
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
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

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
    fn handle<'r>(
        &self,
        db: &'r DbPool,
        request: &'r [u8],
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'r>>;
}

/// Handler that takes care of the process of deserializing the request
/// type and serializing the response type to create a generic [ErasedHandler]
pub struct HandlerBase<H: Handler> {
    _handler: H,
}

impl<H: Handler> ErasedHandler for HandlerBase<H> {
    fn handle<'r>(
        &self,
        db: &'r DbPool,
        request: &'r [u8],
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'r>> {
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
