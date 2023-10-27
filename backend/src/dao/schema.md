# Database schema
This file documents current database schema.

Abbreviations:

abbrev  | description 
------- | -----------
PK      | primary key
AI      | autoincrement
NN      | not null
INT     | integer type
STR     | string type
BIN     | blob type
TIME    | datetime type

## Tables
Each section corresponds to the according table.

### `element`
Main table that stores info about imported elements.

column        | type | modifiers  | description
------------- | ---- | ---------- | -----------
id            | INT  | PK, AI     | element id 
filename      | STR  | NN         | name of file in pool
orig_filename | STR  | NN         | name that file has before import
hash          | BIN  | UNIQUE, NN | md5 hash of file
has_thumb     | INT  | NN         | whether this element has thumbnail
broken        | INT  | NN         | indicates that image lib failed to load this element
animated      | INT  | NN         | whether this element is animation
add_time      | TIME | NN         | time when element was added to DB
file_time     | TIME |            | modification time of element, recorded when it was added to DB


### `tag`
Tag that can be added to element.

column        | type | modifiers  | description
------------- | ---- | ---------- | -----------
id            | INT  | PK, AI     | tag id
tag_name      | STR  | UNIQUE, NN | primary and unique tag name; should be ascii-only for discoverability sake
alt_name      | STR  |            | alternative name of the tag; could be anything
group_id      | INT  |            | id of group this tag belongs to
tag_type      | INT  | NN         | numeric value of the [enum](/common/src/model.rs) identifying tag type 
count         | INT  | NN         | count of elements with this tag
hidden        | INT  | NN         | whether this tag is hidden


### `tag_group`
Used as tag group number sequence

column        | type | modifiers  | description
------------- | ---- | ---------- | -----------
id            | INT  | PK, AI     | sequence value


### `element_tag`
Join table for `element` and `tag`

column        | type | modifiers  | description
------------- | ---- | ---------- | -----------
element_id    | INT  | PK         | id of the corresponding element
tag_id        | INT  | PK         | id of the corresponding tag


### `tag_alias`
This table contains data, that can be fetched from danbooru wiki.
It also stores previous names of tags, so on new imports tags can be mapped to their new names.  
It is also used for translating pixiv tags.

column        | type | modifiers  | description
------------- | ---- | ---------- | -----------
tag_id        | INT  | PK         | id of the corresponding tag
alias         | STR  | NN, UNIQUE | previous/original name of the tag


### `metadata`
Metadata for elements, derived from external source or by parsing element data

column        | type | modifiers  | description
------------- | ---- | ---------- | -----------
element_id    | INT  | PK         | id of the corresponding element
importer_id   | INT  | NN         | id of importer that fetched or parsed the metadata
src_link      | STR  |            | url to element source
src_time      | TIME |            | time when element was added/created on the source 
ext_group     | INT  |            | this value determines external source group, that element corresponds to; it can be arbitrary `i64` value, derived from element
raw_meta      | STR  |            | raw element metadata


### `fetch_status`
Used to determine whether this element can be annoted or already annotated 
with metadata from external source (like pixiv). 

column        | type | modifiers  | description
------------- | ---- | ---------- | -----------
element_id    | INT  | PK         | id of the corresponding element
importer_id   | INT  | PK         | id of importer that will fetch/fetched metadata
failed        | INT  | NN         | flag that set on import fail
supported     | INT  | NN         | whether this import could be done


### `group_metadata`
Metadata for elements, grouped by signature

column        | type | modifiers  | description
------------- | ---- | ---------- | -----------
element_id    | INT  | PK         | id of the corresponding element
signature     | BIN  | NN         | `[i8; 544]`; image signature of the corresponding element 
group_id      | INT  |            | group number


### `group_ids`
Used as signature group number sequence

column        | type | modifiers  | description
------------- | ---- | ---------- | -----------
id            | INT  | PK, AI     | sequence value


## Full SQL script
```sql
create table _sqlx_migrations
(
    version        BIGINT
        primary key,
    description    TEXT                                not null,
    installed_on   TIMESTAMP default CURRENT_TIMESTAMP not null,
    success        BOOLEAN                             not null,
    checksum       BLOB                                not null,
    execution_time BIGINT                              not null
);

create table element
(
    id            INTEGER
        primary key autoincrement,
    filename      TEXT                              not null,
    orig_filename TEXT                              not null,
    hash          BLOB                              not null
        unique,
    has_thumb     INTEGER default 0                 not null,
    broken        INTEGER                           not null,
    animated      INTEGER                           not null,
    add_time      INTEGER default CURRENT_TIMESTAMP not null,
    file_time     INTEGER
);

create table fetch_status
(
    element_id  INTEGER               not null
        references element
            on update restrict on delete cascade,
    importer_id INTEGER               not null,
    failed      INTEGER default FALSE not null,
    supported   INTEGER               not null,
    primary key (element_id, importer_id)
);

create table group_ids
(
    id INTEGER
        primary key autoincrement
);

create table group_metadata
(
    element_id INTEGER not null
        primary key
        references element
            on update restrict on delete cascade,
    signature  BLOB    not null,
    group_id   INTEGER
                       references group_ids
                           on update restrict on delete set null
);

create table metadata
(
    element_id  INTEGER not null
        references element
            on update restrict on delete cascade,
    importer_id INTEGER not null,
    src_link    TEXT,
    src_time    INTEGER,
    ext_group   INTEGER,
    raw_meta    TEXT
);

create table tag_group
(
    id INTEGER not null
        primary key
);

create table tag
(
    id       INTEGER
        primary key autoincrement,
    tag_name TEXT              not null
        unique,
    alt_name TEXT,
    group_id INTEGER
                               references tag_group
                                   on update restrict on delete set null,
    tag_type INTEGER           not null,
    count    INTEGER default 0 not null,
    hidden   INTEGER default 0 not null
);

create table element_tag
(
    element_id INTEGER not null
        references element
            on update restrict on delete cascade,
    tag_id     INTEGER not null
        references tag
            on update cascade on delete cascade,
    primary key (element_id, tag_id)
);

create table tag_alias
(
    tag_id INTEGER not null
        references tag
            on update restrict on delete cascade,
    alias  TEXT    not null
        unique
);
```
