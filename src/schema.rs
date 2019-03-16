table! {
    users (uid) {
        uid -> Oid,
        username -> Varchar,
        email -> Varchar,
        password -> Varchar,
        avatar_url -> Varchar,
        signature -> Varchar,
        created_at -> Timestamp,
    }
}
