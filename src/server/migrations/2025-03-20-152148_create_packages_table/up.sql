create table packages
(
    id                 INTEGER primary key autoincrement NOT NULL,
    name               TEXT              NOT NULL,
    run_before         TEXT    DEFAULT NULL,
    status             SMALLINT DEFAULT 1 NOT NULL,
    last_built         INT8 DEFAULT NULL,
    files              TEXT    DEFAULT '[]' NOT NULL,
    last_built_version TEXT    DEFAULT NULL,
    last_error         TEXT    DEFAULT NULL
);
