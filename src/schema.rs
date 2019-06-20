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
        topic_count -> Int4,
        post_count -> Int4,
        subscriber_count -> Int4,
        thumbnail -> Varchar,
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
    talks (id) {
        id -> Oid,
        name -> Varchar,
        description -> Varchar,
        owner -> Oid,
        admin -> Array<Oid>,
        users -> Array<Oid>,
    }
}

table! {
    topics1 (id) {
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
    topics2 (id) {
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
    topics3 (id) {
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
    topics4 (id) {
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
    topics5 (id) {
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
        show_email -> Bool,
        show_created_at -> Bool,
        show_updated_at -> Bool,
    }
}

allow_tables_to_appear_in_same_query!(
    associates,
    categories,
    posts,
    talks,
    topics1,
    topics2,
    topics3,
    topics4,
    topics5,
    users,
);
