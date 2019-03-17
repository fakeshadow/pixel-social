table! {
    posts (pid) {
        pid -> Oid,
        uid -> Oid,
        topid -> Nullable<Oid>,
        post_content -> Varchar,
    }
}

table! {
    topics (tid) {
        tid -> Oid,
        uid -> Oid,
        mainpid -> Oid,
        topic_content -> Varchar,
    }
}

table! {
    users (uid) {
        uid -> Oid,
        username -> Varchar,
        email -> Varchar,
        password -> Varchar,
        avatar_url -> Varchar,
        signature -> Varchar,
        created_at -> Timestamp,
        is_admin -> Bool,
        blocked -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    posts,
    topics,
    users,
);
