CREATE TABLE IF NOT EXISTS projects
(
    id       BIGSERIAL PRIMARY KEY,
    name     TEXT NOT NULL,
    platform INT  NOT NULL,
    repo_url TEXT NULL
);

CREATE TABLE IF NOT EXISTS sources
(
    id         BIGSERIAL PRIMARY KEY,
    hash       TEXT   NOT NULL,
    project_id BIGINT NOT NULL,
    name       TEXT   NOT NULL,
    filepath   TEXT   NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS hash_idx ON sources (hash);

CREATE TABLE IF NOT EXISTS symbols
(
    id         BIGSERIAL PRIMARY KEY,
    source_id  BIGINT NOT NULL,
    pos        BIGINT NOT NULL,
    name       TEXT   NOT NULL,
    fuzzy_hash BIGINT NOT NULL,
    exact_hash BIGINT NOT NULL,
    FOREIGN KEY (source_id) REFERENCES sources (id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS fuzzy_hash_idx ON symbols (fuzzy_hash);
CREATE INDEX IF NOT EXISTS exact_hash_idx ON symbols (exact_hash);

CREATE TABLE IF NOT EXISTS windows
(
    id        BIGSERIAL PRIMARY KEY,
    symbol_id BIGINT NOT NULL,
    pos       INT    NOT NULL,
    hash      BIGINT NOT NULL,
    FOREIGN KEY (symbol_id) REFERENCES symbols (id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS hash_idx ON windows (hash);