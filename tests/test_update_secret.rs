use aws_sdk_secretsmanager::{
    error::SdkError,
    operation::create_secret::CreateSecretError,
    primitives::Blob,
    types::{
        Tag,
        error::{InvalidRequestException, ResourceExistsException},
    },
};
use uuid::Uuid;

use crate::common::test_server;

mod common;

/// Tests that the description of a secret can be set
#[tokio::test]
async fn test_update_secret_set_description_success() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have no description
    assert_eq!(describe_response.description(), None);

    let update_response = client
        .update_secret()
        .secret_id("test")
        .description("this is the secret description")
        .send()
        .await
        .unwrap();

    // ARN and name should match create
    assert_eq!(update_response.arn(), create_response.arn());
    assert_eq!(update_response.name(), create_response.name());

    // Version ID should not be present as the value was not changed
    assert_eq!(update_response.version_id(), None);

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have the new description
    assert_eq!(
        describe_response.description(),
        Some("this is the secret description")
    );
}

/// Tests that the description of a secret can be updated
#[tokio::test]
async fn test_update_secret_update_description_success() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .description("original description")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have no description
    assert_eq!(
        describe_response.description(),
        Some("original description")
    );

    let update_response = client
        .update_secret()
        .secret_id("test")
        .description("this is the secret description")
        .send()
        .await
        .unwrap();

    // ARN and name should match create
    assert_eq!(update_response.arn(), create_response.arn());
    assert_eq!(update_response.name(), create_response.name());

    // Version ID should not be present as the value was not changed
    assert_eq!(update_response.version_id(), None);

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have the new description
    assert_eq!(
        describe_response.description(),
        Some("this is the secret description")
    );
}
