# Loker

> WIP

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

## Disclaimer

This project is not affiliated with or endorsed by Amazon AWS.
