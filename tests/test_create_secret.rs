use aws_sdk_secretsmanager::{
    error::SdkError,
    operation::create_secret::CreateSecretError,
    primitives::Blob,
    types::{Tag, error::ResourceExistsException},
};
use uuid::Uuid;

use crate::common::test_server;

mod common;

/// Tests that a string secret can be created successfully
#[tokio::test]
async fn test_create_secret_string_success() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    // Server should reply with a version_id for the created version
    assert!(create_response.version_id().is_some());

    // Name should match
    assert_eq!(create_response.name(), Some("test"));

    let get_response = client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Created ARN should match
    assert_eq!(get_response.arn(), create_response.arn());

    // Retrieved value should match created
    assert_eq!(get_response.secret_string(), Some("test"));

    // Retrieved version should match created
    assert_eq!(get_response.version_id(), create_response.version_id());

    // Created secret should be in the AWSCURRENT version stage
    assert_eq!(
        get_response
            .version_stages()
            .first()
            .map(|value| value.as_ref()),
        Some("AWSCURRENT")
    );

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have a matching tag
    assert_eq!(
        describe_response.tags(),
        &[Tag::builder().key("test-tag").value("test-value").build()]
    );
}

/// Tests that a binary secret can be created successfully
#[tokio::test]
async fn test_create_secret_binary_success() {
    let (client, _server) = test_server().await;

    let binary_secret = Blob::new(b"TEST");

    let create_response = client
        .create_secret()
        .name("test")
        .secret_binary(binary_secret.clone())
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    // Server should reply with a version_id for the created version
    assert!(create_response.version_id().is_some());

    // Name should match
    assert_eq!(create_response.name(), Some("test"));

    let get_response = client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Created ARN should match
    assert_eq!(get_response.arn(), create_response.arn());

    // Retrieved value should match created
    assert_eq!(get_response.secret_binary(), Some(&binary_secret));

    // Retrieved version should match created
    assert_eq!(get_response.version_id(), create_response.version_id());

    // Created secret should be in the AWSCURRENT version stage
    assert_eq!(
        get_response
            .version_stages()
            .first()
            .map(|value| value.as_ref()),
        Some("AWSCURRENT")
    );

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have a matching tag
    assert_eq!(
        describe_response.tags(),
        &[Tag::builder().key("test-tag").value("test-value").build()]
    );
}

/// Tests that attempting to create a secret with a name thats already
/// in use will fail with a ResourceExistsException
#[tokio::test]
async fn test_create_secret_duplicate_error() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_error = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap_err();

    let create_error = match create_error {
        SdkError::ServiceError(error) => error,
        error => panic!("expected SdkError::ServiceError got {error:?}"),
    };

    let _exception: ResourceExistsException = match create_error.into_err() {
        CreateSecretError::ResourceExistsException(error) => error,
        error => panic!("expected CreateSecretError::ResourceExistsException got {error:?}"),
    };
}

/// Tests that will simulate a client retrying a request.
///
/// Uses the same secret and the same ClientRequestToken in order
/// to simulate a client sending multiple requests after one failed
///
/// The server should tolerate this without attempting to create
/// additional resources
#[tokio::test]
async fn test_create_secret_client_retry_safety() {
    let (client, _server) = test_server().await;

    let client_request_token = Uuid::new_v4().to_string();

    let create_response_1 = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .client_request_token(client_request_token.clone())
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .client_request_token(client_request_token)
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    assert_eq!(create_response_1.arn(), create_response_2.arn());
    assert_eq!(create_response_1.name(), create_response_2.name());
    assert_eq!(
        create_response_1.version_id(),
        create_response_2.version_id()
    );
    assert_eq!(
        create_response_1.replication_status(),
        create_response_2.replication_status()
    );
}

/// Tests that will simulate trying to create a version that already exists
///
/// Uses the same secret and the same ClientRequestToken for a different secret
///
/// The server should fail this request
#[tokio::test]
async fn test_create_secret_client_duplicate_version_error() {
    let (client, _server) = test_server().await;

    let client_request_token = Uuid::new_v4().to_string();

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .client_request_token(client_request_token.clone())
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_error = client
        .create_secret()
        .name("test")
        .secret_string("test-duplicate")
        .client_request_token(client_request_token)
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap_err();

    let create_error = match create_error {
        SdkError::ServiceError(error) => error,
        error => panic!("expected SdkError::ServiceError got {error:?}"),
    };

    let _exception: ResourceExistsException = match create_error.into_err() {
        CreateSecretError::ResourceExistsException(error) => error,
        error => panic!("expected CreateSecretError::ResourceExistsException got {error:?}"),
    };
}
