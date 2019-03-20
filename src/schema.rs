table! {
    posts (id) {
        id -> Int4,
        user_id -> Int4,
        to_tid -> Int4,
        to_pid -> Int4,
        post_content -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    topics (id) {
        id -> Int4,
        user_id -> Int4,
        title_content -> Varchar,
        post_content -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    users (id) {
        id -> Int4,
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
joinable!(posts -> users (user_id));
joinable!(topics -> users (user_id));

allow_tables_to_appear_in_same_query!(
    posts,
    topics,
    users,
);
