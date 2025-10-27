use crate::common::test_server;
use aws_sdk_secretsmanager::types::Tag;

mod common;

/// Tests that a secret can be described
#[tokio::test]
async fn test_describe_secret_success() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .description("test description")
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

    assert_eq!(describe_response.arn(), create_response.arn());
    assert_eq!(describe_response.name(), create_response.name());

    // Description should match
    assert_eq!(describe_response.description(), Some("test description"));

    // Secret should have a non zero creation and modified date
    assert!(describe_response.created_date().unwrap().secs() > 0);
    assert!(describe_response.last_changed_date().unwrap().secs() > 0);

    // Secret should not have a deleted date
    assert_eq!(describe_response.deleted_date(), None);

    // Secret should not have an accessed date yet
    assert_eq!(describe_response.last_accessed_date(), None);

    // The following should always have defaults
    assert_eq!(describe_response.kms_key_id(), None);
    assert_eq!(describe_response.last_rotated_date(), None);
    assert_eq!(describe_response.next_rotation_date(), None);
    assert_eq!(describe_response.owning_service(), None);
    assert_eq!(describe_response.primary_region(), None);
    assert_eq!(describe_response.replication_status(), &[]);
    assert_eq!(describe_response.rotation_enabled(), Some(false));
    assert_eq!(describe_response.rotation_lambda_arn(), None);
    assert_eq!(describe_response.rotation_rules(), None);

    // Should have matching tags
    assert_eq!(
        describe_response.tags(),
        &[Tag::builder().key("test-tag").value("test-value").build()]
    );
}

/// Tests that accessing a secret changes the last_accessed_date
#[tokio::test]
async fn test_describe_secret_last_accessed_date_success() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .description("test description")
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

    // Secret should not have an accessed date yet
    assert_eq!(describe_response.last_accessed_date(), None);

    // Access the secret
    client
        .get_secret_value()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    let describe_response_1 = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // Secret should have a non zero accessed date
    assert!(describe_response_1.last_accessed_date().unwrap().secs() > 0);

    // Access the secret
    client
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

    // After second access we should have a newer timestamp
    assert!(
        describe_response_2.last_accessed_date().unwrap()
            > describe_response_1.last_accessed_date().unwrap()
    );
}

/// Tests that a initial secret should have the current version in the
/// stages list
#[tokio::test]
async fn test_describe_secret_version_stages_initial_success() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .description("test description")
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

    // Secret should not have an accessed date yet
    assert_eq!(describe_response.last_accessed_date(), None);

    let stages = describe_response.version_ids_to_stages().unwrap();

    // Should only have one version with a stage
    assert_eq!(stages.len(), 1);

    // Current version should be in the current stage
    let current_version_stages = stages.get(create_response.version_id().unwrap()).unwrap();
    assert_eq!(current_version_stages.len(), 1);
    assert_eq!(&current_version_stages[0], "AWSCURRENT");
}

/// Tests that after applying a change to the secret the version stages reflect it
#[tokio::test]
async fn test_describe_secret_version_stages_multiple_success() {
    let (client, _server) = test_server().await;

    let create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .description("test description")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let update_response = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    let stages = describe_response.version_ids_to_stages().unwrap();

    // Should only have one version with a stage
    assert_eq!(stages.len(), 2);

    // Current version should be in the current stage
    let current_version_stages = stages.get(update_response.version_id().unwrap()).unwrap();
    assert_eq!(current_version_stages.len(), 1);
    assert_eq!(&current_version_stages[0], "AWSCURRENT");

    // Previous version should be in the previous stage
    let current_version_stages = stages.get(create_response.version_id().unwrap()).unwrap();
    assert_eq!(current_version_stages.len(), 1);
    assert_eq!(&current_version_stages[0], "AWSPREVIOUS");
}

/// Tests that after applying a change to the secret the last changed will update
#[tokio::test]
async fn test_describe_secret_version_last_changed_success() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .description("test description")
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

    // Secret should have a changed date
    assert!(describe_response_1.last_changed_date().unwrap().secs() > 0);

    let _update_response = client
        .put_secret_value()
        .secret_id("test")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let describe_response_2 = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // After change we should have a newer timestamp
    assert!(
        describe_response_2.last_changed_date().unwrap()
            > describe_response_1.last_changed_date().unwrap()
    );
}

/// Tests that scheduling a deletion will show the deleted date
#[tokio::test]
async fn test_describe_secret_deleted_date_success() {
    let (client, _server) = test_server().await;

    let _create_response = client
        .create_secret()
        .name("test")
        .secret_string("test")
        .description("test description")
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

    let describe_response = client
        .describe_secret()
        .secret_id("test")
        .send()
        .await
        .unwrap();

    // After delete we should have a deleted date
    assert!(describe_response.deleted_date().unwrap().secs() > 0);
}
