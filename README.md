<h1>
    <img src="assets/loker.png" height="128px">
</h1>

**Loker** is a self-hosted AWS secrets manager compatible server. With the main purpose of being used for Integration and End-to-end testing use cases without requiring alternative secret backends.

Data is stored in an encrypted SQLite database using [SQLCipher](https://github.com/sqlcipher/sqlcipher). Server supports using HTTPS and enforces AWS SigV4 signing on requests.

## Implementations:

- [ ] [BatchGetSecretValue](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_BatchGetSecretValue.html)
- [x] [CreateSecret](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_CreateSecret.html)
- [x] [DeleteSecret](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DeleteSecret.html)
- [x] [DescribeSecret](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DescribeSecret.html)
- [ ] [GetRandomPassword](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetRandomPassword.html)
- [x] [GetSecretValue](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetSecretValue.html)
- [ ] [ListSecrets](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ListSecrets.html)
- [ ] [ListSecretVersionIds](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ListSecretVersionIds.html)
- [x] [PutSecretValue](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_PutSecretValue.htmls)
- [x] [RestoreSecret](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_RestoreSecret.html)
- [x] [TagResource](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_TagResource.html)
- [x] [UntagResource](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_UntagResource.html)
- [x] [UpdateSecret](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_UpdateSecret.html)
- [ ] [UpdateSecretVersionStage](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_UpdateSecretVersionStage.html)

\* Implementation awaiting testing

## Not Planned:

- [ ] [CancelRotateSecret](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_CancelRotateSecret.html)
- [ ] [DeleteResourcePolicy](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_DeleteResourcePolicy.html)
- [ ] [GetResourcePolicy](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_GetResourcePolicy.html)
- [ ] [PutResourcePolicy](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_PutResourcePolicy.html)
- [ ] [RemoveRegionsFromReplication](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_RemoveRegionsFromReplication.html)
- [ ] [ReplicateSecretToRegions](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ReplicateSecretToRegions.html)
- [ ] [RotateSecret](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_RotateSecret.html)
- [ ] [StopReplicationToReplica](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_StopReplicationToReplica.html)
- [ ] [ValidateResourcePolicy](https://docs.aws.amazon.com/secretsmanager/latest/apireference/API_ValidateResourcePolicy.html)

## Environment Variables

| Name                      | Required                                           | Description                                            |
| ------------------------- | -------------------------------------------------- | ------------------------------------------------------ |
| SM_ENCRYPTION_KEY         | Yes                                                | Encryption key to encrypt the database with            |
| SM_DATABASE_PATH          | No (Default: secrets.db)                           | Path to the file where the database should be stored   |
| SM_ACCESS_KEY_ID          | Yes                                                | Access key ID to use the server for AWS SigV4          |
| SM_ACCESS_KEY_SECRET      | Yes                                                | Access key secret to use the server for AWS SigV4      |
| SM_SERVER_ADDRESS         | No (Default: HTTP=0.0.0.0:8080 HTTPS=0.0.0.0:8443) | Socket address to bind the server to                   |
| SM_USE_HTTPS              | No (Default: false)                                | Whether to use HTTPS instead of HTTP                   |
| SM_HTTPS_CERTIFICATE_PATH | No (Default: sm.cert.pem)                          | Path to the certificate in PEM format to use for HTTPS |
| SM_HTTPS_PRIVATE_KEY_PATH | No (Default: sm.key.pem)                           | Path to the private key in PEM format to use for HTTPS |

## Windows Build Notes

If you are building on Windows ensure you download the required prerequisites from https://wiki.openssl.org/index.php/Compilation_and_Installation#Windows
**Loker** depends on OpenSSL for the vendored SQLCipher dependency

## Disclaimer

This project is not affiliated with or endorsed by Amazon AWS.
