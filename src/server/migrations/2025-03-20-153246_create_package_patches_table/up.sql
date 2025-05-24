create table package_patches
(
    id  INTEGER primary key autoincrement NOT NULL,
    package_id INTEGER NOT NULL
        constraint table_name_table_name_test_fk
            references packages (id)
            on delete cascade,
    url TEXT NOT NULL,
    sha_512 TEXT DEFAULT NULL
);