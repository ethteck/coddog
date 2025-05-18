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
    name       TEXT   NOT NULL,
    filepath   TEXT   NOT NULL,
    project_id BIGINT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);
CREATE UNIQUE INDEX IF NOT EXISTS hash_idx ON sources (hash);

CREATE TABLE IF NOT EXISTS symbols
(
    id          BIGSERIAL PRIMARY KEY,
    pos         BIGINT NOT NULL,
    name        TEXT   NOT NULL,
    opcode_hash BIGINT NOT NULL,
    equiv_hash  BIGINT NOT NULL,
    exact_hash  BIGINT NOT NULL,
    source_id   BIGINT NOT NULL,
    FOREIGN KEY (source_id) REFERENCES sources (id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS opcode_hash_idx ON symbols (opcode_hash);
CREATE INDEX IF NOT EXISTS equiv_hash_idx ON symbols (equiv_hash);
CREATE INDEX IF NOT EXISTS exact_hash_idx ON symbols (exact_hash);

CREATE TABLE IF NOT EXISTS windows
(
    id        BIGSERIAL PRIMARY KEY,
    pos       INT    NOT NULL,
    hash      BIGINT NOT NULL,
    symbol_id BIGINT NOT NULL,
    FOREIGN KEY (symbol_id) REFERENCES symbols (id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS hash_idx ON windows (hash);
CREATE INDEX IF NOT EXISTS symbol_idx ON windows (symbol_id);
CREATE INDEX IF NOT EXISTS hash_symbol_idx ON windows (hash, symbol_id);