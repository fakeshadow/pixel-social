table! {
    posts (pid) {
        pid -> Int4,
        uid -> Int4,
        to_tid -> Int4,
        to_pid -> Nullable<Int4>,
        post_content -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    topics (tid) {
        tid -> Int4,
        uid -> Int4,
        title_content -> Varchar,
        post_content -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    users (uid) {
        uid -> Int4,
        username -> Varchar,
        email -> Varchar,
        hashed_password -> Varchar,
        avatar_url -> Varchar,
        signature -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        is_admin -> Int4,
        blocked -> Bool,
    }
}

joinable!(posts -> topics (to_tid));
joinable!(posts -> users (uid));
joinable!(topics -> users (uid));

allow_tables_to_appear_in_same_query!(
    posts,
    topics,
    users,
);
