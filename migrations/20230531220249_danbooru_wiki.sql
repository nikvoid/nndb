CREATE TABLE wiki (
    id       INTEGER NOT NULL PRIMARY KEY,
    title    TEXT NOT NULL,
    category INTEGER NOT NULL
);

CREATE TABLE wiki_alias (
    wiki_id  INTEGER NOT NULL,
    alias    TEXT NOT NULL UNIQUE,

    FOREIGN KEY (wiki_id) REFERENCES wiki (id) 
    ON DELETE CASCADE ON UPDATE RESTRICT
);