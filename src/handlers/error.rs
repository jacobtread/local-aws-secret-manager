use axum::{
    Json,
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
};
use serde_json::json;

pub struct InvalidRequestException;

impl IntoResponse for InvalidRequestException {
    fn into_response(self) -> axum::response::Response {
        let body = json!({
            "__type": "InvalidRequestException",
            "message": "A parameter value is not valid for the current state of the resource."
        });

        let mut response = (StatusCode::BAD_REQUEST, Json(body)).into_response();
        response.headers_mut().insert(
            "x-amzn-errortype",
            HeaderValue::from_static("InvalidRequestException"),
        );
        response
    }
}

pub struct ResourceNotFoundException;

impl IntoResponse for ResourceNotFoundException {
    fn into_response(self) -> axum::response::Response {
        let body = json!({
            "__type": "ResourceNotFoundException",
            "message": "Secrets Manager can't find the resource that you asked for."
        });

        let mut response = (StatusCode::BAD_REQUEST, Json(body)).into_response();
        response.headers_mut().insert(
            "x-amzn-errortype",
            HeaderValue::from_static("ResourceNotFoundException"),
        );
        response
    }
}

pub struct ResourceExistsException;

impl IntoResponse for ResourceExistsException {
    fn into_response(self) -> axum::response::Response {
        let body = json!({
            "__type": "ResourceExistsException",
            "message": "A resource with the ID you requested already exists."
        });

        let mut response = (StatusCode::BAD_REQUEST, Json(body)).into_response();
        response.headers_mut().insert(
            "x-amzn-errortype",
            HeaderValue::from_static("ResourceExistsException"),
        );
        response
    }
}

pub struct NotImplemented;

impl IntoResponse for NotImplemented {
    fn into_response(self) -> axum::response::Response {
        let body = json!({
            "__type": "NotImplemented",
            "message": "This operation is not implemented in this server"
        });

        let mut response = (StatusCode::NOT_IMPLEMENTED, Json(body)).into_response();
        response.headers_mut().insert(
            "x-amzn-errortype",
            HeaderValue::from_static("NotImplemented"),
        );
        response
    }
}

pub struct InternalServiceError;

impl IntoResponse for InternalServiceError {
    fn into_response(self) -> axum::response::Response {
        let body = json!({
            "__type": "InternalServiceError",
            "message": "An error occurred on the server side."
        });

        let mut response = (StatusCode::BAD_REQUEST, Json(body)).into_response();
        response.headers_mut().insert(
            "x-amzn-errortype",
            HeaderValue::from_static("InternalServiceError"),
        );
        response
    }
}
