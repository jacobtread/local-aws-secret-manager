use crate::common::test_server;
use aws_sdk_secretsmanager::types::{Filter, Tag};

mod common;

#[tokio::test]
async fn test_list_secrets_all() {
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

    let list_1 = client
        .list_secrets()
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
