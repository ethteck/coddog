CREATE TABLE IF NOT EXISTS projects
(
    id       BIGSERIAL PRIMARY KEY,
    name     TEXT NOT NULL,
    platform INT  NOT NULL
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
CREATE UNIQUE INDEX IF NOT EXISTS hash_unique_idx ON sources (hash);

CREATE TABLE IF NOT EXISTS symbols
(
    id        BIGSERIAL PRIMARY KEY,
    source_id BIGINT NOT NULL,
    "offset"  BIGINT NULL,
    name      TEXT   NULL,
    FOREIGN KEY (source_id) REFERENCES sources (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS hashes
(
    id        BIGSERIAL PRIMARY KEY,
    symbol_id BIGINT NOT NULL,
    hash      BIGINT NOT NULL,
    "offset"  INT    NOT NULL,
    FOREIGN KEY (symbol_id) REFERENCES symbols (id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS hash_idx ON hashes (hash);