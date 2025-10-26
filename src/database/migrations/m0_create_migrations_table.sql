CREATE TABLE IF NOT EXISTS "migrations"
(
    "name"           VARCHAR NOT NULL,
    "applied_at"     DATETIME NOT NULL,

    PRIMARY KEY ("name")
);
