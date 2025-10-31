use crate::common::test_server;
use aws_sdk_secretsmanager::{
    error::SdkError,
    operation::{create_secret::CreateSecretOutput, list_secrets::ListSecretsError},
    types::{Filter, Tag, error::InvalidRequestException},
};

mod common;

/// Tests that the secret list can be obtained in ascending order
#[tokio::test]
async fn test_list_secrets_asc() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-one-match")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::Name)
                .values("test-")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 3);

    let mut secret_list = secret_list.iter();

    let first_secret = secret_list.next().unwrap();
    assert_eq!(first_secret.arn(), create_response_1.arn());
    assert_eq!(first_secret.name(), create_response_1.name());

    let second_secret = secret_list.next().unwrap();
    assert_eq!(second_secret.arn(), create_response_2.arn());
    assert_eq!(second_secret.name(), create_response_2.name());

    let third_secret = secret_list.next().unwrap();
    assert_eq!(third_secret.arn(), create_response_3.arn());
    assert_eq!(third_secret.name(), create_response_3.name());
}

/// Tests that secrets can be found by prefix match for the description
#[tokio::test]
async fn test_list_secrets_description_filter() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-wont-match")
        .description("description-that-wont-match")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::Description)
                .values("test-")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 3);

    let mut secret_list = secret_list.iter();

    let first_secret = secret_list.next().unwrap();
    assert_eq!(first_secret.arn(), create_response_1.arn());
    assert_eq!(first_secret.name(), create_response_1.name());

    let second_secret = secret_list.next().unwrap();
    assert_eq!(second_secret.arn(), create_response_2.arn());
    assert_eq!(second_secret.name(), create_response_2.name());

    let third_secret = secret_list.next().unwrap();
    assert_eq!(third_secret.arn(), create_response_3.arn());
    assert_eq!(third_secret.name(), create_response_3.name());
}

/// Tests that no matches will be found when theres no matching description
#[tokio::test]
async fn test_list_secrets_description_filter_no_matches() {
    let (client, _server) = test_server().await;

    let _create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::Description)
                .values("unknown")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 0);
}

/// Tests that secrets can be found by prefix match for the name
#[tokio::test]
async fn test_list_secrets_name_filter() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-wont-match")
        .description("description-that-wont-match")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::Name)
                .values("test-")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 3);

    let mut secret_list = secret_list.iter();

    let first_secret = secret_list.next().unwrap();
    assert_eq!(first_secret.arn(), create_response_1.arn());
    assert_eq!(first_secret.name(), create_response_1.name());

    let second_secret = secret_list.next().unwrap();
    assert_eq!(second_secret.arn(), create_response_2.arn());
    assert_eq!(second_secret.name(), create_response_2.name());

    let third_secret = secret_list.next().unwrap();
    assert_eq!(third_secret.arn(), create_response_3.arn());
    assert_eq!(third_secret.name(), create_response_3.name());
}

/// Tests that no matches will be found when theres no matching name
#[tokio::test]
async fn test_list_secrets_name_filter_no_matches() {
    let (client, _server) = test_server().await;

    let _create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag").value("test-value").build())
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::Name)
                .values("unknown")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 0);
}

/// Tests that secrets can be found by prefix match for an associated tag key
#[tokio::test]
async fn test_list_secrets_tag_key_filter() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag-1").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag-2").value("test-value").build())
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag-3").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-wont-match")
        .description("description-that-wont-match")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("no-match-test-tag-4")
                .value("test-value")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::TagKey)
                .values("test-tag")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 3);

    let mut secret_list = secret_list.iter();

    let first_secret = secret_list.next().unwrap();
    assert_eq!(first_secret.arn(), create_response_1.arn());
    assert_eq!(first_secret.name(), create_response_1.name());

    let second_secret = secret_list.next().unwrap();
    assert_eq!(second_secret.arn(), create_response_2.arn());
    assert_eq!(second_secret.name(), create_response_2.name());

    let third_secret = secret_list.next().unwrap();
    assert_eq!(third_secret.arn(), create_response_3.arn());
    assert_eq!(third_secret.name(), create_response_3.name());
}

/// Tests that no matches will be found when theres no matching tag key
#[tokio::test]
async fn test_list_secrets_tag_key_filter_no_matches() {
    let (client, _server) = test_server().await;

    let _create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag-1").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag-2").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(Tag::builder().key("test-tag-3").value("test-value").build())
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-wont-match")
        .description("description-that-wont-match")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("no-match-test-tag-4")
                .value("test-value")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::TagKey)
                .values("unknown")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 0);
}

/// Tests that secrets can be found by prefix match for an associated tag value
#[tokio::test]
async fn test_list_secrets_tag_value_filter() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-1")
                .value("test-value-1")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-wont-match")
        .description("description-that-wont-match")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("no-match-test-tag-4")
                .value("no-match-test-value-4")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::TagValue)
                .values("test-value-")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 3);

    let mut secret_list = secret_list.iter();

    let first_secret = secret_list.next().unwrap();
    assert_eq!(first_secret.arn(), create_response_1.arn());
    assert_eq!(first_secret.name(), create_response_1.name());

    let second_secret = secret_list.next().unwrap();
    assert_eq!(second_secret.arn(), create_response_2.arn());
    assert_eq!(second_secret.name(), create_response_2.name());

    let third_secret = secret_list.next().unwrap();
    assert_eq!(third_secret.arn(), create_response_3.arn());
    assert_eq!(third_secret.name(), create_response_3.name());
}

/// Tests that no matches will be found when theres no matching tag value
#[tokio::test]
async fn test_list_secrets_tag_value_filter_no_matches() {
    let (client, _server) = test_server().await;

    let _create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-1")
                .value("test-value-1")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-wont-match")
        .description("description-that-wont-match")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("no-match-test-tag-4")
                .value("no-match-test-value-4")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::TagValue)
                .values("unknown")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 0);
}

/// Tests that matches for all an be found
#[tokio::test]
async fn test_list_secrets_all_filter() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-1")
                .value("test-value-1")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-wont-match")
        .description("description-that-wont-match")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("no-match-test-tag-4")
                .value("no-match-test-value-4")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::All)
                .values("test")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 3);

    let mut secret_list = secret_list.iter();

    let first_secret = secret_list.next().unwrap();
    assert_eq!(first_secret.arn(), create_response_1.arn());
    assert_eq!(first_secret.name(), create_response_1.name());

    let second_secret = secret_list.next().unwrap();
    assert_eq!(second_secret.arn(), create_response_2.arn());
    assert_eq!(second_secret.name(), create_response_2.name());

    let third_secret = secret_list.next().unwrap();
    assert_eq!(third_secret.arn(), create_response_3.arn());
    assert_eq!(third_secret.name(), create_response_3.name());
}

/// Tests that bad all matches will have no results
#[tokio::test]
async fn test_list_secrets_all_filter_no_matches() {
    let (client, _server) = test_server().await;

    let _create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-1")
                .value("test-value-1")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-wont-match")
        .description("description-that-wont-match")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("no-match-test-tag-4")
                .value("no-match-test-value-4")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::All)
                .values("unknown")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 0);
}

/// Tests that combining a name and tag filter together will yield more specific results
#[tokio::test]
async fn test_list_secrets_name_and_tag_filter() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-1")
                .value("test-value-1")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("not-matching-test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-wont-match")
        .description("description-that-wont-match")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("no-match-test-tag-4")
                .value("no-match-test-value-4")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::Name)
                .values("test")
                .build(),
        )
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::TagKey)
                .values("test")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 2);

    let mut secret_list = secret_list.iter();

    let first_secret = secret_list.next().unwrap();
    assert_eq!(first_secret.arn(), create_response_1.arn());
    assert_eq!(first_secret.name(), create_response_1.name());

    let second_secret = secret_list.next().unwrap();
    assert_eq!(second_secret.arn(), create_response_2.arn());
    assert_eq!(second_secret.name(), create_response_2.name());
}

/// Tests that combining a name and tag filter together will yield no results
/// when theres no secrets that meet the criteria
#[tokio::test]
async fn test_list_secrets_name_and_tag_filter_no_matches() {
    let (client, _server) = test_server().await;

    let _create_response_1 = client
        .create_secret()
        .name("test-1")
        .description("test-1")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-1")
                .value("test-value-1")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_2 = client
        .create_secret()
        .name("test-2")
        .description("test-2")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_3 = client
        .create_secret()
        .name("test-3")
        .description("test-3")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("not-matching-test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("name-that-wont-match")
        .description("description-that-wont-match")
        .secret_string("test")
        .tags(
            Tag::builder()
                .key("no-match-test-tag-4")
                .value("no-match-test-value-4")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::Name)
                .values("unknown")
                .build(),
        )
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::TagKey)
                .values("unknown")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_list = list_1.secret_list();
    assert_eq!(secret_list.len(), 0);
}

/// Prefixing a filter value with ! should invert the filter to instead exclude the value
#[tokio::test]
async fn test_list_secrets_negation_filter() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .secret_string("test-1")
        .tags(
            Tag::builder()
                .key("test-tag-1")
                .value("test-value-1")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .secret_string("test-2")
        .tags(
            Tag::builder()
                .key("test-tag-2")
                .value("test-value-2")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .secret_string("test-3")
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let _create_response_4 = client
        .create_secret()
        .name("test-4")
        .secret_string("test-4")
        .tags(
            Tag::builder()
                .key("test-tag-4")
                .value("test-value-4")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let secrets = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::TagValue)
                .values("test-")
                .build(),
        )
        // Exclude test-4 from the results
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::TagValue)
                .values("!test-value-4")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let secret_values = secrets.secret_list();
    assert_eq!(secret_values.len(), 3);

    let mut secret_values = secret_values.iter();

    let secret_1 = secret_values.next().unwrap();
    assert_eq!(secret_1.arn(), create_response_3.arn());
    assert_eq!(secret_1.name(), create_response_3.name());

    let secret_2 = secret_values.next().unwrap();
    assert_eq!(secret_2.arn(), create_response_2.arn());
    assert_eq!(secret_2.name(), create_response_2.name());

    let secret_3 = secret_values.next().unwrap();
    assert_eq!(secret_3.arn(), create_response_1.arn());
    assert_eq!(secret_3.name(), create_response_1.name());
}

/// Tests that requesting a list before creating any secrets will return an empty list
#[tokio::test]
async fn test_list_secrets_empty_list() {
    let (client, _server) = test_server().await;

    let list_1 = client
        .list_secrets()
        .filters(
            Filter::builder()
                .key(aws_sdk_secretsmanager::types::FilterNameStringType::Description)
                .values("test-")
                .build(),
        )
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    assert!(list_1.secret_list().is_empty());
    assert_eq!(list_1.next_token(), None);
}

/// Tests that requesting a list with no filters will provide all secrets that are
/// not pending deletion in DESC order
#[tokio::test]
async fn test_list_secrets_no_filters_list() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .secret_string("test-1")
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .secret_string("test-3")
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_4 = client
        .create_secret()
        .name("test-4")
        .secret_string("test-4")
        .send()
        .await
        .unwrap();

    client
        .delete_secret()
        .secret_id(create_response_4.arn().unwrap())
        .send()
        .await
        .unwrap();

    let secrets = client.list_secrets().send().await.unwrap();

    let secret_values = secrets.secret_list();
    assert_eq!(secret_values.len(), 3);

    let mut secret_values = secret_values.iter();

    let secret_1 = secret_values.next().unwrap();
    assert_eq!(secret_1.arn(), create_response_3.arn());
    assert_eq!(secret_1.name(), create_response_3.name());

    let secret_2 = secret_values.next().unwrap();
    assert_eq!(secret_2.arn(), create_response_2.arn());
    assert_eq!(secret_2.name(), create_response_2.name());

    let secret_3 = secret_values.next().unwrap();
    assert_eq!(secret_3.arn(), create_response_1.arn());
    assert_eq!(secret_3.name(), create_response_1.name());
}

/// Tests that requesting a list with no filters with the asc sort order
/// will provide all secrets that are not pending deletion in ASC order
#[tokio::test]
async fn test_list_secrets_no_filters_asc_list() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .secret_string("test-1")
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .secret_string("test-3")
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_4 = client
        .create_secret()
        .name("test-4")
        .secret_string("test-4")
        .send()
        .await
        .unwrap();

    client
        .delete_secret()
        .secret_id(create_response_4.arn().unwrap())
        .send()
        .await
        .unwrap();

    let secrets = client
        .list_secrets()
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_values = secrets.secret_list();
    assert_eq!(secret_values.len(), 3);

    let mut secret_values = secret_values.iter();

    let secret_1 = secret_values.next().unwrap();
    assert_eq!(secret_1.arn(), create_response_1.arn());
    assert_eq!(secret_1.name(), create_response_1.name());

    let secret_2 = secret_values.next().unwrap();
    assert_eq!(secret_2.arn(), create_response_2.arn());
    assert_eq!(secret_2.name(), create_response_2.name());

    let secret_3 = secret_values.next().unwrap();
    assert_eq!(secret_3.arn(), create_response_3.arn());
    assert_eq!(secret_3.name(), create_response_3.name());
}

/// Tests that requesting a list with no filters will provide all secrets
/// including those pending deletion in DESC order
#[tokio::test]
async fn test_list_secrets_no_filters_include_pending_deletion_list() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .secret_string("test-1")
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .secret_string("test-3")
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_4 = client
        .create_secret()
        .name("test-4")
        .secret_string("test-4")
        .send()
        .await
        .unwrap();

    client
        .delete_secret()
        .secret_id(create_response_4.arn().unwrap())
        .send()
        .await
        .unwrap();

    let secrets = client
        .list_secrets()
        .include_planned_deletion(true)
        .send()
        .await
        .unwrap();

    let secret_values = secrets.secret_list();
    assert_eq!(secret_values.len(), 4);

    let mut secret_values = secret_values.iter();

    let secret_1 = secret_values.next().unwrap();
    assert_eq!(secret_1.arn(), create_response_4.arn());
    assert_eq!(secret_1.name(), create_response_4.name());

    let secret_2 = secret_values.next().unwrap();
    assert_eq!(secret_2.arn(), create_response_3.arn());
    assert_eq!(secret_2.name(), create_response_3.name());

    let secret_3 = secret_values.next().unwrap();
    assert_eq!(secret_3.arn(), create_response_2.arn());
    assert_eq!(secret_3.name(), create_response_2.name());

    let secret_4 = secret_values.next().unwrap();
    assert_eq!(secret_4.arn(), create_response_1.arn());
    assert_eq!(secret_4.name(), create_response_1.name());
}

/// Tests that requesting a list with no filters with the asc sort order
/// will provide all secrets including those pending deletion in ASC order
#[tokio::test]
async fn test_list_secrets_no_filters_include_pending_deletion_asc_list() {
    let (client, _server) = test_server().await;

    let create_response_1 = client
        .create_secret()
        .name("test-1")
        .secret_string("test-1")
        .send()
        .await
        .unwrap();

    let create_response_2 = client
        .create_secret()
        .name("test-2")
        .secret_string("test-2")
        .send()
        .await
        .unwrap();

    let create_response_3 = client
        .create_secret()
        .name("test-3")
        .secret_string("test-3")
        .tags(
            Tag::builder()
                .key("test-tag-3")
                .value("test-value-3")
                .build(),
        )
        .send()
        .await
        .unwrap();

    let create_response_4 = client
        .create_secret()
        .name("test-4")
        .secret_string("test-4")
        .send()
        .await
        .unwrap();

    client
        .delete_secret()
        .secret_id(create_response_4.arn().unwrap())
        .send()
        .await
        .unwrap();

    let secrets = client
        .list_secrets()
        .include_planned_deletion(true)
        .sort_order(aws_sdk_secretsmanager::types::SortOrderType::Asc)
        .send()
        .await
        .unwrap();

    let secret_values = secrets.secret_list();
    assert_eq!(secret_values.len(), 4);

    let mut secret_values = secret_values.iter();

    let secret_1 = secret_values.next().unwrap();
    assert_eq!(secret_1.arn(), create_response_1.arn());
    assert_eq!(secret_1.name(), create_response_1.name());

    let secret_2 = secret_values.next().unwrap();
    assert_eq!(secret_2.arn(), create_response_2.arn());
    assert_eq!(secret_2.name(), create_response_2.name());

    let secret_3 = secret_values.next().unwrap();
    assert_eq!(secret_3.arn(), create_response_3.arn());
    assert_eq!(secret_3.name(), create_response_3.name());

    let secret_3 = secret_values.next().unwrap();
    assert_eq!(secret_3.arn(), create_response_4.arn());
    assert_eq!(secret_3.name(), create_response_4.name());
}

/// Tests that the default pagination size works correctly
#[tokio::test]
async fn test_list_secrets_default_pagination() {
    let (client, _server) = test_server().await;

    let page_size = 100;
    let pages = 3;
    let items_to_make = page_size * pages;

    let mut created_secrets: Vec<CreateSecretOutput> = Vec::new();

    for i in 0..items_to_make {
        let secret = client
            .create_secret()
            .name(format!("test-{i}"))
            .secret_string(format!("test-{i}"))
            .send()
            .await
            .unwrap();
        created_secrets.push(secret);
    }

    created_secrets.reverse();

    let mut next_token: Option<String> = None;

    for page in 0..pages {
        let secrets = client
            .list_secrets()
            .filters(
                Filter::builder()
                    .key(aws_sdk_secretsmanager::types::FilterNameStringType::Name)
                    .values("test-")
                    .build(),
            )
            .set_next_token(next_token)
            .send()
            .await
            .unwrap();

        for i in 0..page_size {
            let secret = secrets.secret_list().get(i).unwrap();
            let created_secret = created_secrets.get(i + (page_size * page)).unwrap();

            // Make sure the versions match
            assert_eq!(secret.arn(), created_secret.arn());
            assert_eq!(secret.name(), created_secret.name());
        }

        if page < pages - 1 {
            assert_eq!(
                secrets.next_token(),
                Some(format!("{}:{}", page_size, page + 1).as_str())
            );

            next_token = secrets.next_token().map(|value| value.to_string());
        } else {
            // Should have nothing more
            assert_eq!(secrets.next_token(), None);
            next_token = None;
        }
    }
}

/// Tests that the custom pagination size works correctly
#[tokio::test]
async fn test_list_secrets_custom_pagination() {
    let (client, _server) = test_server().await;

    let page_size = 50;
    let pages = 3;
    let items_to_make = page_size * pages;

    let mut created_secrets: Vec<CreateSecretOutput> = Vec::new();

    for i in 0..items_to_make {
        let secret = client
            .create_secret()
            .name(format!("test-{i}"))
            .secret_string(format!("test-{i}"))
            .send()
            .await
            .unwrap();
        created_secrets.push(secret);
    }

    created_secrets.reverse();

    let mut next_token: Option<String> = None;

    for page in 0..pages {
        let secrets = client
            .list_secrets()
            .filters(
                Filter::builder()
                    .key(aws_sdk_secretsmanager::types::FilterNameStringType::Name)
                    .values("test-")
                    .build(),
            )
            .max_results(page_size as i32)
            .set_next_token(next_token)
            .send()
            .await
            .unwrap();

        for i in 0..page_size {
            let secret = secrets.secret_list().get(i).unwrap();
            let created_secret = created_secrets.get(i + (page_size * page)).unwrap();

            // Make sure the versions match
            assert_eq!(secret.arn(), created_secret.arn());
            assert_eq!(secret.name(), created_secret.name());
        }

        if page < pages - 1 {
            assert_eq!(
                secrets.next_token(),
                Some(format!("{}:{}", page_size, page + 1).as_str())
            );

            next_token = secrets.next_token().map(|value| value.to_string());
        } else {
            // Should have nothing more
            assert_eq!(secrets.next_token(), None);
            next_token = None;
        }
    }
}

/// Tests that requesting an invalid pagination token should error
#[tokio::test]
async fn test_list_secrets_invalid_pagination_error() {
    let (client, _server) = test_server().await;
    let list_error = client
        .list_secrets()
        .next_token("test_122")
        .send()
        .await
        .unwrap_err();

    let list_error = match list_error {
        SdkError::ServiceError(error) => error,
        error => panic!("expected SdkError::ServiceError got {error:?}"),
    };

    let _exception: InvalidRequestException = match list_error.into_err() {
        ListSecretsError::InvalidRequestException(error) => error,
        error => panic!("expected BatchGetSecretValueError::InvalidRequestException got {error:?}"),
    };
}
