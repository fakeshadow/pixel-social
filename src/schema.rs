table! {
    categories (id) {
        id -> Int4,
        name -> Varchar,
        theme -> Varchar,
    }
}

table! {
    posts (id) {
        id -> Int4,
        user_id -> Int4,
        topic_id -> Int4,
        post_id -> Int4,
        post_content -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
    }
}

table! {
    topics (id) {
        id -> Int4,
        user_id -> Int4,
        category_id -> Int4,
        title -> Varchar,
        body -> Varchar,
        thumbnail -> Varchar,
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

joinable!(posts -> topics (topic_id));
joinable!(posts -> users (user_id));
joinable!(topics -> categories (category_id));
joinable!(topics -> users (user_id));

allow_tables_to_appear_in_same_query!(
    categories,
    posts,
    topics,
    users,
);
