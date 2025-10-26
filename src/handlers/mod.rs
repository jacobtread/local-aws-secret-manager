use crate::database::DbPool;
use axum::{
    Json,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{collections::HashMap, pin::Pin};

pub mod create_secret;
pub mod delete_secret;
pub mod describe_secret;
pub mod error;
pub mod get_secret_value;
pub mod list_secrets;
pub mod put_secret_value;
pub mod update_secret;

#[derive(Deserialize)]
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
}

pub trait Handler: Send + Sync + 'static {
    type Request: DeserializeOwned + Send + 'static;
    type Response: Serialize + Send + 'static;

    fn handle<'d>(
        db: &'d DbPool,
        request: Self::Request,
    ) -> impl Future<Output = Result<Self::Response, Response>> + Send + 'd;
}

pub trait ErasedHandler: Send + Sync + 'static {
    fn handle<'r>(
        &self,
        db: &'r DbPool,
        request: &'r [u8],
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'r>>;
}

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
            let request: H::Request = serde_json::from_slice(request).unwrap();
            match H::handle(db, request).await {
                Ok(response) => Json(response).into_response(),
                Err(error) => error,
            }
        })
    }
}
