use axum::{
    Json,
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
};
use serde_json::json;

pub trait AwsError {
    const STATUS_CODE: StatusCode = StatusCode::BAD_REQUEST;
    const TYPE: &str;
    const MESSAGE: &str;
}

pub struct AwsErrorResponse<A: AwsError>(pub A);

impl<A: AwsError> IntoResponse for AwsErrorResponse<A> {
    fn into_response(self) -> axum::response::Response {
        let body = json!({
            "__type": A::TYPE,
            "message": A::MESSAGE
        });

        let mut response = (A::STATUS_CODE, Json(body)).into_response();
        response
            .headers_mut()
            .insert("x-amzn-errortype", HeaderValue::from_static(A::TYPE));
        response
    }
}

pub struct InvalidClientTokenId;

impl AwsError for InvalidClientTokenId {
    const STATUS_CODE: StatusCode = StatusCode::FORBIDDEN;
    const TYPE: &str = "InvalidClientTokenId";
    const MESSAGE: &str =
        "The X.509 certificate or AWS access key ID provided does not exist in our records.";
}

pub struct SignatureDoesNotMatch;

impl AwsError for SignatureDoesNotMatch {
    const STATUS_CODE: StatusCode = StatusCode::FORBIDDEN;
    const TYPE: &str = "SignatureDoesNotMatch";
    const MESSAGE: &str = "The request signature we calculated does not match the signature you provided. Check your AWS Secret Access Key and signing method. Consult the service documentation for details.";
}

pub struct MissingAuthenticationToken;

impl AwsError for MissingAuthenticationToken {
    const TYPE: &str = "MissingAuthenticationToken";
    const MESSAGE: &str = "Missing Authentication Token";
}

pub struct IncompleteSignature;

impl AwsError for IncompleteSignature {
    const TYPE: &str = "IncompleteSignature";
    const MESSAGE: &str = "The request signature does not conform to AWS standards.";
}

pub struct InvalidRequestException;

impl AwsError for InvalidRequestException {
    const TYPE: &str = "InvalidRequestException";
    const MESSAGE: &str = "A parameter value is not valid for the current state of the resource.";
}

pub struct InvalidParameterException;

impl AwsError for InvalidParameterException {
    const TYPE: &str = "InvalidParameterException";
    const MESSAGE: &str = "The parameter name or value is invalid.";
}

pub struct ResourceNotFoundException;

impl AwsError for ResourceNotFoundException {
    const TYPE: &str = "ResourceNotFoundException";
    const MESSAGE: &str = "Secrets Manager can't find the resource that you asked for.";
}

pub struct ResourceExistsException;

impl AwsError for ResourceExistsException {
    const TYPE: &str = "ResourceExistsException";
    const MESSAGE: &str = "A resource with the ID you requested already exists.";
}

pub struct NotImplemented;

impl AwsError for NotImplemented {
    const TYPE: &str = "ResourceExistsException";
    const MESSAGE: &str = "This operation is not implemented in this server";
}

pub struct InternalServiceError;

impl AwsError for InternalServiceError {
    const TYPE: &str = "InternalServiceError";
    const MESSAGE: &str = "An error occurred on the server side.";
}
