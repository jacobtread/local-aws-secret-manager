CREATE TABLE IF NOT EXISTS "secrets" (
    -- Secret ARN
    "arn" TEXT PRIMARY KEY NOT NULL,

    -- Metadata
    "name" TEXT NOT NULL,
    "description" TEXT NULL,

    -- Timestamps
    "created_at" DATETIME NOT NULL,
    "updated_at" DATETIME NULL,

    -- Datetime the resource was marked for deletion and the datetime its scheduled to be deleted by
    "delete_at" DATETIME NULL,
    "scheduled_delete_at" DATETIME NULL,

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
    "version_stage" TEXT NOT NULL,

    -- Secret Value
    "secret_string" TEXT NULL,
    "secret_binary" TEXT NULL,

    -- Timestamps
    "created_at" DATETIME NOT NULL,
    "last_accessed_at" DATETIME NULL,

    -- Composite primary key
    PRIMARY KEY ("secret_arn", "version_id"),

    -- Foreign key to "secrets"
    FOREIGN KEY ("secret_arn") REFERENCES "secrets"("arn") ON DELETE CASCADE
);

-- Fast lookups by ARN + Version ID
CREATE INDEX IF NOT EXISTS "idx_secrets_versions_version_id" ON "secrets_versions"("secret_arn", "version_id");
-- Fast lookups by ARN + Version Stage
CREATE INDEX IF NOT EXISTS "idx_secrets_versions_version_stage" ON "secrets_versions"("secret_arn", "version_stage");

CREATE TABLE IF NOT EXISTS "secrets_tags" (
    -- Secret details
    "secret_arn" TEXT NOT NULL,

    -- Tag data
    "key" TEXT NOT NULL,
    "value" TEXT NOT NULL,

    -- Timestamps
    "created_at" DATETIME NOT NULL,
    "updated_at" DATETIME NULL,

    -- Composite primary key
    PRIMARY KEY ("secret_arn", "key"),

    -- Foreign key to "secrets"
    FOREIGN KEY ("secret_arn") REFERENCES "secrets"("arn") ON DELETE CASCADE
);

-- Fast lookups by ARN
CREATE INDEX IF NOT EXISTS "idx_secrets_tags_version_id" ON "secrets_tags"("secret_arn");
