use crate::common::test_server;

mod common;

#[tokio::test]
async fn test_get_random_password_default() {
    let (client, _server) = test_server().await;

    let response = client.get_random_password().send().await.unwrap();
    let password = response.random_password().unwrap();

    // Default length is 32
    assert_eq!(password.len(), 32);
}
#[tokio::test]
async fn test_get_random_password_length() {
    let (client, _server) = test_server().await;

    let response = client
        .get_random_password()
        .password_length(48)
        .send()
        .await
        .unwrap();
    let password = response.random_password().unwrap();

    // Length should match
    assert_eq!(password.len(), 48);
}
