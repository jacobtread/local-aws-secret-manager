CREATE TABLE IF NOT EXISTS "secrets" (
    -- Secret ARN
    "arn" TEXT PRIMARY KEY NOT NULL,

    -- Metadata
    "name" TEXT NOT NULL,
    "description" TEXT NULL,

    -- Timestamps
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NULL,

    -- Datetime the resource was marked for deletion and the datetime its scheduled to be deleted by
    "deleted_at" TEXT NULL,
    "scheduled_delete_at" TEXT NULL,

    -- Name must be unique
    UNIQUE ("name")
);


-- Fast lookups by ARN
CREATE INDEX IF NOT EXISTS "idx_secrets_name" ON "secrets"("arn");

-- Fast lookups by name
CREATE INDEX IF NOT EXISTS "idx_secrets_name" ON "secrets"("name");

CREATE TABLE IF NOT EXISTS "secrets_versions" (
    -- Secret details
    "secret_arn" TEXT NOT NULL,
    "version_id" TEXT NOT NULL,

    -- Secret Value
    "secret_string" TEXT NULL,
    "secret_binary" TEXT NULL,

    -- Timestamps
    "created_at" TEXT NOT NULL,
    "last_accessed_at" TEXT NULL,

    -- Composite primary key
    PRIMARY KEY ("secret_arn", "version_id"),

    -- Foreign key to "secrets"
    FOREIGN KEY ("secret_arn") REFERENCES "secrets"("arn") ON DELETE CASCADE
);

-- Fast lookups by ARN + Version ID
CREATE INDEX IF NOT EXISTS "idx_secrets_versions_version_id" ON "secrets_versions"("secret_arn", "version_id");

CREATE TABLE IF NOT EXISTS "secret_version_stages" (
    -- Version details
    "secret_arn" TEXT NOT NULL,
    "version_id" TEXT NOT NULL,

    -- Stage value
    "value" TEXT NOT NULL,

    "created_at" TEXT NOT NULL,

    -- Each version can have multiple stages but each stage value should be unique per version
    PRIMARY KEY ("secret_arn", "version_id", "value"),

    -- Stage value must be unique for each secret
    UNIQUE ("secret_arn", "value"),

    -- Foreign key referencing secrets_versions composite key
    FOREIGN KEY ("secret_arn", "version_id")
        REFERENCES "secrets_versions"("secret_arn", "version_id")
        ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS "secrets_tags" (
    -- Secret details
    "secret_arn" TEXT NOT NULL,

    -- Tag data
    "key" TEXT NOT NULL,
    "value" TEXT NOT NULL,

    -- Timestamps
    "created_at" TEXT NOT NULL,
    "updated_at" TEXT NULL,

    -- Composite primary key
    PRIMARY KEY ("secret_arn", "key"),

    -- Foreign key to "secrets"
    FOREIGN KEY ("secret_arn") REFERENCES "secrets"("arn") ON DELETE CASCADE
);

-- Fast lookups by ARN
CREATE INDEX IF NOT EXISTS "idx_secrets_tags_version_id" ON "secrets_tags"("secret_arn");
