CREATE TABLE IF NOT EXISTS projects
(
    id   BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    repo TEXT NULL
);

CREATE TABLE IF NOT EXISTS versions
(
    id         BIGSERIAL PRIMARY KEY,
    name       TEXT   NOT NULL,
    platform   INT    NOT NULL,
    project_id BIGINT NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS objects
(
    id         BIGSERIAL PRIMARY KEY,
    hash       TEXT NOT NULL,
    local_path TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS hash_idx ON objects (hash);

CREATE TABLE IF NOT EXISTS sources
(
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT   NOT NULL,
    source_link TEXT   NULL,
    object_id   BIGINT NOT NULL,
    version_id  BIGINT NULL,
    project_id  BIGINT NOT NULL,
    FOREIGN KEY (object_id) REFERENCES objects (id) ON DELETE CASCADE,
    FOREIGN KEY (version_id) REFERENCES versions (id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES projects (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS symbols
(
    id            BIGSERIAL PRIMARY KEY,
    slug          TEXT    NOT NULL,
    len           INT     NOT NULL,
    name          TEXT    NOT NULL,
    is_decompiled BOOLEAN NOT NULL DEFAULT FALSE,
    symbol_idx    INT     NOT NULL,
    opcode_hash   BIGINT  NOT NULL,
    equiv_hash    BIGINT  NOT NULL,
    exact_hash    BIGINT  NOT NULL,
    source_id     BIGINT  NOT NULL,
    FOREIGN KEY (source_id) REFERENCES sources (id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS opcode_hash_idx ON symbols (opcode_hash);
CREATE INDEX IF NOT EXISTS equiv_hash_idx ON symbols (equiv_hash);
CREATE INDEX IF NOT EXISTS exact_hash_idx ON symbols (exact_hash);
CREATE UNIQUE INDEX IF NOT EXISTS symbols_slug_idx ON symbols (slug);

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

-- Trigger function to generate a random slug
CREATE OR REPLACE FUNCTION generate_random_slug(length INTEGER) RETURNS TEXT AS
$$
DECLARE
    chars  TEXT    := 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    result TEXT    := '';
    i      INTEGER := 0;
BEGIN
    FOR i IN 1..length
        LOOP
            result := result || substr(chars, floor(random() * length(chars) + 1)::integer, 1);
        END LOOP;
    RETURN result;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION set_symbol_slug() RETURNS TRIGGER AS
$$
DECLARE
    slug_length INTEGER := 5;
    new_slug    TEXT;
    slug_exists INTEGER;
BEGIN
    -- Generate a new slug and check if it already exists (loop until unique)
    LOOP
        new_slug := generate_random_slug(slug_length);
        SELECT COUNT(*) INTO slug_exists FROM symbols WHERE slug = new_slug;
        EXIT WHEN slug_exists = 0;
    END LOOP;

    NEW.slug := new_slug;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create the trigger
CREATE TRIGGER set_symbol_slug_trigger
    BEFORE INSERT
    ON symbols
    FOR EACH ROW
EXECUTE FUNCTION set_symbol_slug();
