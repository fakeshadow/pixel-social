table! {
    associates (id) {
        id -> Oid,
        user_id -> Oid,
        psn_id -> Nullable<Varchar>,
        live_id -> Nullable<Varchar>,
        last_update_time -> Timestamp,
    }
}

table! {
    categories (id) {
        id -> Oid,
        name -> Varchar,
        theme -> Varchar,
    }
}

table! {
    posts (id) {
        id -> Oid,
        user_id -> Oid,
        topic_id -> Oid,
        post_id -> Nullable<Oid>,
        post_content -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        last_reply_time -> Timestamp,
        reply_count -> Int4,
        is_locked -> Bool,
    }
}

table! {
    topics (id) {
        id -> Oid,
        user_id -> Oid,
        category_id -> Oid,
        title -> Varchar,
        body -> Varchar,
        thumbnail -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        last_reply_time -> Timestamp,
        reply_count -> Int4,
        is_locked -> Bool,
    }
}

table! {
    users (id) {
        id -> Oid,
        username -> Varchar,
        email -> Varchar,
        hashed_password -> Varchar,
        avatar_url -> Varchar,
        signature -> Varchar,
        created_at -> Timestamp,
        updated_at -> Timestamp,
        is_admin -> Oid,
        blocked -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    associates,
    categories,
    posts,
    topics,
    users,
);
