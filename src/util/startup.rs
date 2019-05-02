use crate::model::{
    common::{PostgresPool, RedisPool, GlobalVar, GlobalGuard, match_id}
};
use crate::handler::{
    category::load_all_categories,
    user::{get_last_uid, load_all_users},
    post::{get_last_pid, load_all_posts_with_topic_id},
    topic::{get_last_tid, get_topic_list},
    cache::{update_cache, build_list, update_meta},
};
use crate::model::errors::ServiceError;

// ToDo: build category ranks and topic ranks at startup;
pub fn build_cache(db_pool: &PostgresPool, cache_pool: &RedisPool) -> Result<(), ()> {
    let conn = &db_pool.get().unwrap_or_else(|_| panic!("Database is offline"));
    let conn_cache = cache_pool.get().unwrap_or_else(|_| panic!("Cache is offline"));

    /// Load all categories and make hash set.
    let categories = load_all_categories(conn).unwrap_or_else(|_| panic!("Failed to load categories"));
    update_cache(&categories, "category", conn_cache).unwrap_or_else(|_| panic!("Failed to update categories hash set"));

    println!("Categories loaded");

    /// build list by last reply time desc order for each category. build category meta list with all category ids
    let conn_cache = cache_pool.get().unwrap_or_else(|_| panic!("Cache is offline"));
    let mut meta_ids = Vec::new();
    for cat in categories.iter() {
        meta_ids.push(cat.id);
        let topic_list = get_topic_list(&cat.id, conn).unwrap_or_else(|_| panic!("Failed to build category lists"));
        build_list(topic_list, &format!("category:{}", &cat.id), &conn_cache).unwrap_or_else(|_| panic!("Failed to build category lists"));
    }
    update_meta(meta_ids, "category_id", &conn_cache).unwrap_or_else(|_| panic!("Failed to build category meta"));
    println!("Category list and meta data loaded");

    /// Load all posts with topic id and build a list of posts for each topic
    let posts = load_all_posts_with_topic_id(&conn).unwrap_or_else(|_| panic!("Failed to load posts"));
    let mut temp = Vec::new();
    let mut index: u32 = posts[0].0;
    for post in posts.into_iter() {
        let (i, v) = post;
        if i == index {
            temp.push(v)
        } else {
            build_list(temp, &format!("topic:{}", index), &conn_cache).unwrap_or_else(|_| panic!("Failed to build category lists"));
            temp = Vec::new();
            index = i;
            temp.push(v);
        }
    }
    build_list(temp, &format!("topic:{}", index), &conn_cache).unwrap_or_else(|_| panic!("Failed to build category lists"));
    println!("Post list loaded");

    /// load all users and store the data in a zrange. stringify user data as member, user id as score.
    let users = load_all_users(conn).unwrap_or_else(|_| panic!("Failed to load users"));
    update_cache(&users, "user", conn_cache).unwrap_or_else(|_| panic!("Failed to update users cache"));
    println!("User cache loaded. Cache built success");

    Ok(())
}

pub fn init_global_var(pool: &PostgresPool) -> GlobalGuard {
    let conn = pool.get().unwrap();
    let next_uid = match_id(get_last_uid(&conn));
    let next_pid = match_id(get_last_pid(&conn));
    let next_tid = match_id(get_last_tid(&conn));
    println!("GlobalState loaded");
    GlobalVar::new(next_uid, next_pid, next_tid)
}