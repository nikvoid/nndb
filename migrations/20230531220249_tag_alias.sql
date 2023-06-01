CREATE TABLE tag_alias (
    tag_hash INTEGER NOT NULL,
    alias    TEXT NOT NULL UNIQUE,

    FOREIGN KEY (tag_hash) REFERENCES tag (name_hash) 
    ON DELETE CASCADE ON UPDATE RESTRICT
);