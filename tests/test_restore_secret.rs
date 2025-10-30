use aws_sdk_secretsmanager::{
    error::SdkError,
    operation::restore_secret::RestoreSecretError,
    types::{Tag, error::ResourceNotFoundException},
};

use crate::common::test_server;

mod common;

/// Tests that requesting a scheduled deletion succeeds
#[tokio::test]
async fn test_restore_secret_success() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _delete_response = client
        .delete_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    let restore_response = client
        .restore_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Ensure correct response
    assert_eq!(restore_response.arn(), create_response.arn());
    assert_eq!(restore_response.name(), create_response.name());

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Secret should not longer have a deletion date
    assert_eq!(describe_response.deleted_date, None);
}

/// Tests that restoring a secret thats not scheduled for deletion will
/// not error
#[tokio::test]
async fn test_restore_secret_not_scheduled_success() {}

/// Tests that trying to restore an unknown secret will fail
#[tokio::test]
async fn test_restore_secret_unknown_error() {
    let (client, _server) = test_server().await;

    let restore_err = client
        .restore_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap_err();

    let restore_err = match restore_err {
        SdkError::ServiceError(error) => error,
        error => panic!("expected SdkError::ServiceError got {error:?}"),
    };

    let _exception: ResourceNotFoundException = match restore_err.into_err() {
        RestoreSecretError::ResourceNotFoundException(error) => error,
        error => panic!("expected RestoreSecretError::ResourceNotFoundException got {error:?}"),
    };
}
