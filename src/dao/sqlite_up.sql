CREATE TABLE IF NOT EXISTS element (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    filename     TEXT NOT NULL,
    orig_name    TEXT NOT NULL,
    -- md5 blob of size 16 bytes
    hash         BLOB NOT NULL UNIQUE,
    has_thumb    INTEGER NOT NULL DEFAULT 0,
    broken       INTEGER NOT NULL,
    animated     INTEGER NOT NULL,
    add_time     INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- table for pending element imports processing
CREATE TABLE IF NOT EXISTS import (
    element_id   INTEGER PRIMARY KEY NOT NULL,
    importer_id  INTEGER NOT NULL,
    
    FOREIGN KEY (element_id) REFERENCES element (id) ON DELETE CASCADE ON UPDATE RESTRICT
);

CREATE TABLE IF NOT EXISTS group_ids (
    id INTEGER PRIMARY KEY AUTOINCREMENT
);

-- TODO: is it better to make table for many-to-many joining similar elements?..
-- pros: may reduce anomalies
-- cons: may make grouping more strict (that is undesired...)
-- table for element group metadata
CREATE TABLE IF NOT EXISTS group_metadata (
    element_id   INTEGER PRIMARY KEY NOT NULL,
    -- blob of fixed size 544 (may be changed later)
    signature    BLOB NOT NULL,
    group_id     INTEGER,

    FOREIGN KEY (element_id) REFERENCES element   (id) ON DELETE CASCADE  ON UPDATE RESTRICT,
    FOREIGN KEY (group_id)   REFERENCES group_ids (id) ON DELETE SET NULL ON UPDATE RESTRICT
);

-- Also a marker that element was processed
CREATE TABLE IF NOT EXISTS metadata (
    element_id   INTEGER PRIMARY KEY NOT NULL,
    src_link     TEXT,
    src_time     INTEGER,
    -- this field intended to use with external group information source
    ext_group    INTEGER,
    
    FOREIGN KEY (element_id) REFERENCES element (id) ON DELETE CASCADE ON UPDATE RESTRICT
);

CREATE TABLE IF NOT EXISTS ai_metadata (
    element_id      INTEGER PRIMARY KEY NOT NULL,
    positive_prompt TEXT NOT NULL,
    negative_prompt TEXT,
    steps           INTEGER NOT NULL,
    scale           REAL NOT NULL,
    sampler         TEXT NOT NULL,
    seed            INTEGER NOT NULL,
    strength        REAL NOT NULL,
    noise           REAL NOT NULL,

    FOREIGN KEY (element_id) REFERENCES element (id) ON DELETE CASCADE ON UPDATE RESTRICT
);

CREATE TABLE IF NOT EXISTS tag (
    -- crc32 hash of tag name
    name_hash INTEGER PRIMARY KEY NOT NULL,
    tag_name  TEXT NOT NULL UNIQUE,
    alt_name  TEXT,
    -- id of alias group, NULL for tag not in group
    group_id  INTEGER,
    tag_type  INTEGER NOT NULL,
    hidden    INTEGER NOT NULL DEFAULT 0
);

-- join table for element and tag
CREATE TABLE IF NOT EXISTS element_tag (
    element_id INTEGER NOT NULL,
    tag_hash   INTEGER NOT NULL,

    FOREIGN KEY (element_id) REFERENCES element (id)
        ON DELETE CASCADE
        ON UPDATE RESTRICT,
    FOREIGN KEY (tag_hash) REFERENCES tag (name_hash)
        ON DELETE CASCADE
        ON UPDATE CASCADE,

    PRIMARY KEY (element_id, tag_hash)
);