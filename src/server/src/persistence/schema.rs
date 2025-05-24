// @generated automatically by Diesel CLI.

diesel::table! {
    package_patches (id) {
        id -> Integer,
        package_id -> Integer,
        url -> Text,
        sha_512 -> Nullable<Text>,
    }
}

diesel::table! {
    packages (id) {
        id -> Integer,
        name -> Text,
        run_before -> Nullable<Text>,
        status -> SmallInt,
        last_built -> Nullable<BigInt>,
        files -> Text,
        last_built_version -> Nullable<Text>,
        last_error -> Nullable<Text>,
    }
}

diesel::joinable!(package_patches -> packages (package_id));

diesel::allow_tables_to_appear_in_same_query!(
    package_patches,
    packages,
);
