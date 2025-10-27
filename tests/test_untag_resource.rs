use crate::common::test_server;
use aws_sdk_secretsmanager::{
    error::SdkError,
    operation::untag_resource::UntagResourceError,
    types::{Tag, error::ResourceNotFoundException},
};

mod common;

/// Tests that a tag can be removed from a secret
#[tokio::test]
async fn test_untag_resource_success() {
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
        .untag_resource()
        .secret_id("test")
        .tag_keys("test-tag")
        .send()
        .await
        .unwrap();

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have no tags
    assert_eq!(describe_response.tags(), &[]);
}

/// Tests that multiple tags can be removed from a resource
#[tokio::test]
async fn test_untag_resource_multiple_success() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .tags(Tag::builder().key("test-tag-1").value("test-value").build())
        .tags(Tag::builder().key("test-tag-2").value("test-value").build())
        .send()
        .await
        .unwrap();

    client
        .untag_resource()
        .secret_id("test")
        .tag_keys("test-tag-1")
        .tag_keys("test-tag-2")
        .send()
        .await
        .unwrap();

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should ony have the remaining tag
    assert_eq!(
        describe_response.tags(),
        &[Tag::builder().key("test-tag").value("test-value").build(),]
    );
}

/// Tests that removing non existent tags does not cause an error
#[tokio::test]
async fn test_untag_resource_unknown_tags() {
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
        .untag_resource()
        .secret_id("test")
        .tag_keys("test-tag-no-exist")
        .tag_keys("test-tag-no-exist-2")
        .send()
        .await
        .unwrap();

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Should have a matching tag
    assert_eq!(
        describe_response.tags(),
        &[Tag::builder().key("test-tag").value("test-value").build(),]
    );
}

/// Tests that trying to remove a tag from an unknown resource will fail
#[tokio::test]
async fn test_untag_resource_unknown_error() {
    let (client, _server) = test_server().await;

    let tag_err = client
        .untag_resource()
        .secret_id("test")
        .tag_keys("test-tag")
        .send()
        .await
        .unwrap_err();

    let tag_err = match tag_err {
        SdkError::ServiceError(error) => error,
        error => panic!("expected SdkError::ServiceError got {error:?}"),
    };

    let _exception: ResourceNotFoundException = match tag_err.into_err() {
        UntagResourceError::ResourceNotFoundException(error) => error,
        error => panic!("expected UntagResourceError::ResourceNotFoundException got {error:?}"),
    };
}
