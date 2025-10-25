use crate::{
    json::{JsonSecretManager, JsonSecretManagerConfig, Secret, SecretValue},
    logging::init_logging,
};
use axum::{
    Extension, Json, Router,
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use axum_server::tls_rustls::RustlsConfig;
use bytes::Bytes;
use serde::Deserialize;
use serde_json::json;
use std::{
    error::Error,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
};
use tower_http::trace::TraceLayer;

mod json;
mod logging;

/// Default server address when not specified (HTTP)
const DEFAULT_SERVER_ADDRESS_HTTP: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8080));

/// Default server address when not specified (HTTPS)
const DEFAULT_SERVER_ADDRESS_HTTPS: SocketAddr =
    SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 8443));

fn main() -> Result<(), Box<dyn Error>> {
    _ = dotenvy::dotenv();

    init_logging();

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed building the Runtime")
        .block_on(async move {
            if let Err(error) = server().await {
                tracing::error!(?error, message = %error, "error running server");
                return Err(error);
            }

            Ok(())
        })
}

async fn server() -> Result<(), Box<dyn Error>> {
    let secrets = JsonSecretManager::from_config(JsonSecretManagerConfig::from_env()?);

    // Setup router
    let app = router()
        .layer(Extension(secrets))
        .layer(TraceLayer::new_for_http());

    // Determine whether to use https
    let use_https = match std::env::var("SM_USE_HTTPS") {
        Ok(value) => value.parse::<bool>()?,
        // Default max file size in bytes (100MB)
        Err(_) => false,
    };

    // Determine the socket address to bind against
    let server_address = std::env::var("SERVER_ADDRESS")
        .ok()
        .and_then(|value| value.parse::<SocketAddr>().ok())
        .unwrap_or(if use_https {
            DEFAULT_SERVER_ADDRESS_HTTPS
        } else {
            DEFAULT_SERVER_ADDRESS_HTTP
        });

    // Development mode CORS access for local browser testing
    #[cfg(debug_assertions)]
    let app = app.layer(tower_http::cors::CorsLayer::very_permissive());

    // Log the startup message
    tracing::debug!("server started on {server_address}");

    let handle = axum_server::Handle::default();

    // Handle graceful shutdown on CTRL+C
    tokio::spawn({
        let handle = handle.clone();
        async move {
            _ = tokio::signal::ctrl_c().await;
            handle.graceful_shutdown(None);
        }
    });

    if use_https {
        // Determine whether to use https
        let certificate_path = match std::env::var("SM_HTTPS_CERTIFICATE_PATH") {
            Ok(value) => value,
            Err(_) => "sm.cert.pem".to_string(),
        };

        let private_key_path = match std::env::var("SM_HTTPS_PRIVATE_KEY_PATH") {
            Ok(value) => value,
            Err(_) => "sm.key.pem".to_string(),
        };

        if rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .is_err()
        {
            tracing::error!("failed install default crypto provider");
            return Err(std::io::Error::other("failed to install default crypto provider").into());
        }

        let config = match RustlsConfig::from_pem_file(certificate_path, private_key_path).await {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error, "failed to initialize https config");
                return Err(error.into());
            }
        };

        // Serve the app over HTTPS
        axum_server::bind_rustls(server_address, config)
            .handle(handle)
            .serve(app.into_make_service())
            .await?;
    } else {
        // Serve the app over HTTP
        axum_server::bind(server_address)
            .handle(handle)
            .serve(app.into_make_service())
            .await?;
    }

    Ok(())
}

pub fn router() -> Router {
    Router::new().route("/", post(handle_post))
}

#[derive(Deserialize)]
struct CreateSecretRequest {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "SecretString")]
    secret_string: Option<String>,
    #[serde(rename = "SecretBinary")]
    secret_binary: Option<Vec<u8>>,
}

#[derive(Deserialize)]
struct UpdateSecretRequest {
    #[serde(rename = "SecretId")]
    secret_id: String,
    #[serde(rename = "SecretString")]
    secret_string: Option<String>,
    #[serde(rename = "SecretBinary")]
    secret_binary: Option<Vec<u8>>,
}

#[derive(Deserialize)]
struct GetSecretRequest {
    #[serde(rename = "SecretId")]
    secret_id: String,
}

async fn handle_post(
    Extension(secrets): Extension<JsonSecretManager>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        // TODO: Handle missing target
        .unwrap();

    match target {
        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_CreateSecret.html
        "secretsmanager.CreateSecret" => {
            let request: CreateSecretRequest = serde_json::from_slice(&body).unwrap();

            let arn = format!(
                "arn:aws:secretsmanager:us-east-1:1:secret:{}",
                &request.name
            );

            let secret_value = match (request.secret_string, request.secret_binary) {
                (Some(value), _) => SecretValue::String(value),
                (_, Some(value)) => SecretValue::Binary(value),
                _ => todo!("missing secret"),
            };

            let secret = Secret {
                value: secret_value,
            };

            secrets.set_secret(&request.name, secret).await.unwrap();

            (Json(json!({
                "ARN": arn,
                "Name": request.name,
            })),)
                .into_response()
        }

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DeleteSecret.html
        "secretsmanager.DeleteSecret" => not_implemented_response(),

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DescribeSecret.html
        "secretsmanager.DescribeSecret" => not_implemented_response(),

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html
        "secretsmanager.GetSecretValue" => {
            let request: GetSecretRequest = serde_json::from_slice(&body).unwrap();
            let arn = format!(
                "arn:aws:secretsmanager:us-east-1:1:secret:{}",
                &request.secret_id
            );

            // TODO: Handle get by ARN

            match secrets.get_secret(&request.secret_id).await.unwrap() {
                Some(secret) => match secret.value {
                    json::SecretValue::String(secret) => Json(json!({
                        "ARN": arn,
                        "Name": &request.secret_id,
                        "SecretString": secret,
                        "SecretBinary": serde_json::Value::Null,
                    }))
                    .into_response(),
                    json::SecretValue::Binary(items) => Json(json!({
                        "ARN": arn,
                        "Name": &request.secret_id,
                        "SecretString": serde_json::Value::Null,
                        "SecretBinary": items
                    }))
                    .into_response(),
                },
                None => Json(json!({
                    "ARN": arn,
                    "Name": &request.secret_id,
                    "SecretString": serde_json::Value::Null,
                    "SecretBinary": serde_json::Value::Null,
                }))
                .into_response(),
            }
        }

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ListSecrets.html
        "secretsmanager.ListSecrets" => not_implemented_response(),

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_PutSecretValue.html
        "secretsmanager.PutSecretValue" => not_implemented_response(),

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_UpdateSecret.html
        "secretsmanager.UpdateSecret" => {
            let request: UpdateSecretRequest = serde_json::from_slice(&body).unwrap();

            // TODO: Handle get by ARN

            let arn = format!(
                "arn:aws:secretsmanager:us-east-1:1:secret:{}",
                &request.secret_id
            );

            let secret_value = match (request.secret_string, request.secret_binary) {
                (Some(value), _) => SecretValue::String(value),
                (_, Some(value)) => SecretValue::Binary(value),
                _ => todo!("missing secret"),
            };

            let secret = Secret {
                value: secret_value,
            };

            secrets
                .set_secret(&request.secret_id, secret)
                .await
                .unwrap();

            Json(json!({
                "ARN": arn,
                "Name": request.secret_id,
            }))
            .into_response()
        }

        _ => not_implemented_response(),
    }
}

fn not_implemented_response() -> Response {
    let body = json!({
        "__type": "NotImplemented",
        "message": "This operation is not implemented in this server"
    });

    let mut response = (StatusCode::NOT_IMPLEMENTED, Json(body)).into_response();
    response.headers_mut().insert(
        "x-amzn-errortype",
        HeaderValue::from_static("NotImplementedException"),
    );
    response
}
