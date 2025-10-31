use aws_sdk_secretsmanager::{
    error::SdkError,
    operation::get_secret_value::GetSecretValueError,
    primitives::Blob,
    types::{
        Tag,
        error::{InvalidRequestException, ResourceNotFoundException},
    },
};

use crate::common::test_server;

mod common;

/// Tests that a string secret can be retrieved by name successfully
#[tokio::test]
async fn test_get_secret_value_by_name_string_success() {
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
}

/// Tests that a string secret can be retrieved by name with version successfully
#[tokio::test]
async fn test_get_secret_value_by_name_with_version_string_success() {
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
        .version_id(create_response.version_id().unwrap())
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
}

/// Tests that a string secret can be retrieved by ARN successfully
#[tokio::test]
async fn test_get_secret_value_by_arn_string_success() {
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
        .secret_id(create_response.arn().unwrap())
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
}

/// Tests that a string secret can be retrieved by ARN using a specific version successfully
#[tokio::test]
async fn test_get_secret_value_by_arn_with_version_string_success() {
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
        .secret_id(create_response.arn().unwrap())
        .version_id(create_response.version_id().unwrap())
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
}

/// Tests that a string secret can be retrieved by ARN using a specific version successfully
/// when multiple are present
#[tokio::test]
async fn test_get_secret_value_by_arn_with_version_when_multiple_string_success() {
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

    client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-3")
        .send()
        .await
        .unwrap();

    let get_response = client
        .get_secret_value()
        .secret_id(create_response.arn().unwrap())
        .version_id(create_response.version_id().unwrap())
        .send()
        .await
        .unwrap();

    // Created ARN should match
    assert_eq!(get_response.arn(), create_response.arn());

    // Retrieved value should match created
    assert_eq!(get_response.secret_string(), Some("test"));

    // Retrieved version should match created
    assert_eq!(get_response.version_id(), create_response.version_id());

    // The original created secret should be in the deprecated/None version stage
    assert_eq!(get_response.version_stages().first(), None);
}

/// Tests that a string secret can be retrieved by ARN using the AWSCURRENT version stage
/// successfully when multiple are present
#[tokio::test]
async fn test_get_secret_value_by_arn_with_current_stage_when_multiple_string_success() {
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

    client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let latest_response = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-3")
        .send()
        .await
        .unwrap();

    let get_response = client
        .get_secret_value()
        .secret_id(create_response.arn().unwrap())
        .version_stage("AWSCURRENT")
        .send()
        .await
        .unwrap();

    // Created ARN should match
    assert_eq!(get_response.arn(), create_response.arn());

    // Retrieved value should match the latest
    assert_eq!(get_response.secret_string(), Some("test-3"));

    // Retrieved version should match the latest
    assert_eq!(get_response.version_id(), latest_response.version_id());

    // Created secret should be in the AWSCURRENT version stage
    assert_eq!(
        get_response
            .version_stages()
            .first()
            .map(|value| value.as_ref()),
        Some("AWSCURRENT")
    );
}

/// Tests that a string secret can be retrieved by ARN using a the AWSPREVIOUS version stage
/// successfully when multiple are present
///
/// The most recent previous secret version should be retrieved
#[tokio::test]
async fn test_get_secret_value_by_arn_with_previous_stage_when_multiple_string_success() {
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

    let previous_response = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let _last_response = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-3")
        .send()
        .await
        .unwrap();

    let get_response = client
        .get_secret_value()
        .secret_id(create_response.arn().unwrap())
        .version_stage("AWSPREVIOUS")
        .send()
        .await
        .unwrap();

    // Created ARN should match
    assert_eq!(get_response.arn(), create_response.arn());

    // Retrieved value should match the previous version
    assert_eq!(get_response.secret_string(), Some("test-2"));

    // Retrieved version should match the previous version
    assert_eq!(get_response.version_id(), previous_response.version_id());

    // Created secret should be in the AWSPREVIOUS version stage
    assert_eq!(
        get_response
            .version_stages()
            .first()
            .map(|value| value.as_ref()),
        Some("AWSPREVIOUS")
    );
}

/// Tests that a binary secret can be created successfully
#[tokio::test]
async fn test_get_secret_value_binary_success() {
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
}

/// Tests that requesting an unknown secret will error
#[tokio::test]
async fn test_get_secret_value_by_name_unknown_error() {
    let (client, _server) = test_server().await;

    let get_error = client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap_err();

    let get_error = match get_error {
        SdkError::ServiceError(error) => error,
        error => panic!("expected SdkError::ServiceError got {error:?}"),
    };

    let _exception: ResourceNotFoundException = match get_error.into_err() {
        GetSecretValueError::ResourceNotFoundException(error) => error,
        error => panic!("expected GetSecretValueError::ResourceNotFoundException got {error:?}"),
    };
}

/// Tests that the response should error if the secret is scheduled
/// for deletion
#[tokio::test]
async fn test_get_secret_value_should_error_if_scheduled_for_deletion() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    client
        .delete_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    let get_error = client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap_err();

    let get_error = match get_error {
        SdkError::ServiceError(error) => error,
        error => panic!("expected SdkError::ServiceError got {error:?}"),
    };

    let _exception: InvalidRequestException = match get_error.into_err() {
        GetSecretValueError::InvalidRequestException(error) => error,
        error => panic!("expected GetSecretValueError::InvalidRequestException got {error:?}"),
    };
}

/// Tests that after a secret has been retrieved successfully the last
/// accessed date should be updated
#[tokio::test]
async fn test_get_secret_value_last_accessed_updated() {
    let (client, _server) = test_server().await;

    let binary_secret = Blob::new(b"TEST");

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_binary(binary_secret.clone())
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let describe_response_1 = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    assert_eq!(describe_response_1.last_accessed_date(), None);

    let _get_response = client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    let describe_response_2 = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    assert!(
        describe_response_2
            .last_accessed_date()
            .is_some_and(|value| value.secs() > 0)
    );

    let _get_response = client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    let describe_response_3 = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    assert!(
        describe_response_3.last_accessed_date().unwrap()
            > describe_response_2.last_accessed_date().unwrap()
    );
}
