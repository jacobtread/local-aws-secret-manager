use crate::{
    database::{
        DbPool, create_database,
        secrets::{
            CreateSecret, CreateSecretVersion, VersionStage, create_secret, create_secret_version,
            get_secret_by_version_id, get_secret_by_version_stage,
            get_secret_by_version_stage_and_id, get_secret_latest_version,
            mark_secret_versions_previous, put_secret_tag, update_secret_description,
            update_secret_version_last_accessed,
        },
    },
    handlers::create_secret::{CreateSecretRequest, handle_create_secret},
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
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    error::Error,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    ops::DerefMut,
};
use tower_http::trace::TraceLayer;
use uuid::Uuid;

mod database;
mod handlers;
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

    tokio::runtime::Builder::new_current_thread()
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
    // Encryption key
    let encryption_key = std::env::var("SM_ENCRYPTION_KEY").inspect_err(|_| {
        tracing::error!("Must specify SM_ENCRYPTION_KEY environment variable");
    })?;

    // Path to the database file
    let database_path =
        std::env::var("SM_DATABASE_PATH").unwrap_or_else(|_| "secrets.db".to_string());

    // Connect to or create an encrypted database file
    let pool = create_database(encryption_key, database_path).await?;

    // Setup router
    let app = router()
        .layer(Extension(pool))
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
struct PutSecretValueRequest {
    #[serde(rename = "ClientRequestToken")]
    client_request_token: Option<String>,
    #[serde(rename = "SecretId")]
    secret_id: String,
    #[serde(rename = "SecretString")]
    secret_string: Option<String>,
    #[serde(rename = "SecretBinary")]
    secret_binary: Option<Vec<u8>>,
    #[serde(rename = "VersionStages")]
    version_stages: Vec<String>,
}

#[derive(Serialize)]
struct PutSecretValueResponse {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "VersionId")]
    version_id: String,
    #[serde(rename = "VersionStages")]
    version_stages: Vec<VersionStage>,
}

#[derive(Deserialize)]
struct UpdateSecretRequest {
    #[serde(rename = "ClientRequestToken")]
    client_request_token: Option<String>,
    #[serde(rename = "Description")]
    description: Option<String>,
    #[serde(rename = "SecretId")]
    secret_id: String,
    #[serde(rename = "SecretString")]
    secret_string: Option<String>,
    #[serde(rename = "SecretBinary")]
    secret_binary: Option<Vec<u8>>,
}

#[derive(Serialize)]
struct UpdateSecretResponse {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "VersionId")]
    version_id: Option<String>,
}

#[derive(Deserialize)]
struct Tag {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Value")]
    value: String,
}

#[derive(Deserialize)]
struct GetSecretValueRequest {
    #[serde(rename = "SecretId")]
    secret_id: String,
    #[serde(rename = "VersionId")]
    version_id: Option<String>,
    #[serde(rename = "VersionStage")]
    version_stage: Option<String>,
}

#[derive(Serialize)]
struct GetSecretValueResponse {
    #[serde(rename = "ARN")]
    arn: String,
    #[serde(rename = "CreatedDate")]
    created_date: i64,
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "SecretString")]
    secret_string: Option<String>,
    #[serde(rename = "SecretBinary")]
    secret_binary: Option<Vec<u8>>,
    #[serde(rename = "VersionId")]
    version_id: String,
    #[serde(rename = "VersionStages")]
    version_stages: Vec<VersionStage>,
}

async fn handle_post(
    Extension(db): Extension<DbPool>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let target = headers
        .get("x-amz-target")
        .and_then(|v| v.to_str().ok())
        // TODO: Handle missing target
        .unwrap();

    match target {
        "secretsmanager.CreateSecret" => {
            let request: CreateSecretRequest = serde_json::from_slice(&body).unwrap();
            match handle_create_secret(&db, request).await {
                Ok(response) => Json(response).into_response(),
                Err(response) => response,
            }
        }

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DeleteSecret.html
        "secretsmanager.DeleteSecret" => not_implemented_response(),

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DescribeSecret.html
        "secretsmanager.DescribeSecret" => not_implemented_response(),

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html
        "secretsmanager.GetSecretValue" => {
            let request: GetSecretValueRequest = serde_json::from_slice(&body).unwrap();

            let secret_id = request.secret_id;
            let version_id = request.version_id;
            let version_stage = request
                .version_stage
                .map(VersionStage::try_from)
                .transpose()
                .expect("todo: handle unknown version stage");

            let secret = match (&version_id, version_stage) {
                (None, None) => get_secret_latest_version(&db, &secret_id).await.unwrap(),
                (Some(version_id), Some(version_stage)) => {
                    get_secret_by_version_stage_and_id(&db, &secret_id, version_id, version_stage)
                        .await
                        .unwrap()
                }
                (Some(version_id), None) => get_secret_by_version_id(&db, &secret_id, version_id)
                    .await
                    .unwrap(),
                (None, Some(version_stage)) => {
                    get_secret_by_version_stage(&db, &secret_id, version_stage)
                        .await
                        .unwrap()
                }
            };

            let secret = match secret {
                Some(value) => value,
                None => return not_found_response(),
            };

            update_secret_version_last_accessed(&db, &secret.arn, &secret.version_id)
                .await
                .unwrap();

            let created_at = if version_id.is_some() {
                secret.version_created_at
            } else {
                secret.created_at
            };

            Json(GetSecretValueResponse {
                arn: secret.arn,
                created_date: created_at.timestamp(),
                name: secret.name,
                secret_string: secret.secret_string,
                secret_binary: secret.secret_binary,
                version_id: secret.version_id,
                version_stages: vec![secret.version_stage],
            })
            .into_response()
        }

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ListSecrets.html
        "secretsmanager.ListSecrets" => not_implemented_response(),

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_PutSecretValue.html
        "secretsmanager.PutSecretValue" => {
            let request: PutSecretValueRequest = serde_json::from_slice(&body).unwrap();

            let version_id = request
                .client_request_token
                // Generate a new version ID if none was provided
                .unwrap_or_else(|| Uuid::new_v4().to_string());

            let version_stages: Vec<VersionStage> = request
                .version_stages
                .into_iter()
                // TODO: Handle unsupported?
                .filter_map(|version| VersionStage::try_from(version).ok())
                .collect();

            let version_stage = version_stages
                .first()
                .copied()
                .unwrap_or(VersionStage::Current);

            if request.secret_string.is_none() && request.secret_binary.is_none() {
                todo!("missing secret error")
            }

            let secret_id = request.secret_id;

            let secret = get_secret_latest_version(&db, &secret_id).await.unwrap();
            let secret = match secret {
                Some(value) => value,
                None => return not_found_response(),
            };

            let mut t = db.begin().await.unwrap();

            if matches!(version_stage, VersionStage::Current) {
                // Mark previous versions as non current
                mark_secret_versions_previous(t.deref_mut(), &secret.arn)
                    .await
                    .unwrap();
            }

            // Create the initial secret version
            create_secret_version(
                t.deref_mut(),
                CreateSecretVersion {
                    secret_arn: secret.arn.clone(),
                    version_id: version_id.clone(),
                    version_stage,
                    secret_string: request.secret_string,
                    secret_binary: request.secret_binary,
                },
            )
            .await
            .unwrap();

            t.commit().await.unwrap();

            // TODO: Handle unique constraint violation for version ID

            Json(PutSecretValueResponse {
                arn: secret.arn,
                name: secret.name,
                version_id: secret.version_id,
                version_stages: vec![secret.version_stage],
            })
            .into_response()
        }

        // https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_UpdateSecret.html
        "secretsmanager.UpdateSecret" => {
            let request: UpdateSecretRequest = serde_json::from_slice(&body).unwrap();

            let secret_id = request.secret_id;

            let secret = get_secret_latest_version(&db, &secret_id).await.unwrap();
            let secret = match secret {
                Some(value) => value,
                None => return not_found_response(),
            };

            let mut t = db.begin().await.unwrap();

            if let Some(description) = request.description {
                update_secret_description(t.deref_mut(), &secret.arn, &description)
                    .await
                    .unwrap();
            }

            let version_id = if request.secret_string.is_some() || request.secret_binary.is_some() {
                let version_id = request
                    .client_request_token
                    // Generate a new version ID if none was provided
                    .unwrap_or_else(|| Uuid::new_v4().to_string());

                // Mark previous versions as non current
                mark_secret_versions_previous(t.deref_mut(), &secret.arn)
                    .await
                    .unwrap();

                // Create a new current secret version
                create_secret_version(
                    t.deref_mut(),
                    CreateSecretVersion {
                        secret_arn: secret.arn.clone(),
                        version_id: version_id.clone(),
                        version_stage: VersionStage::Current,
                        secret_string: request.secret_string,
                        secret_binary: request.secret_binary,
                    },
                )
                .await
                .unwrap();

                Some(version_id)
            } else {
                None
            };

            t.commit().await.unwrap();

            Json(UpdateSecretResponse {
                arn: secret.arn,
                name: secret.name,
                version_id,
            })
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
        HeaderValue::from_static("NotImplemented"),
    );
    response
}

fn not_found_response() -> Response {
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
