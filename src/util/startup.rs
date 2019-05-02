use crate::model::{
    common::{PostgresPool, RedisPool, GlobalVar, GlobalGuard, match_id}
};
use crate::handler::{
    category::load_all_categories,
    user::{get_last_uid, load_all_users},
    post::get_last_pid,
    topic::{get_last_tid, get_topic_list},
    cache::{update_cache, build_list, update_meta},
};
use crate::model::errors::ServiceError;

// ToDo: build category ranks and topic ranks at startup;
pub fn build_cache(db_pool: &PostgresPool, cache_pool: &RedisPool) -> Result<(), ()> {
    let conn = &db_pool.get().unwrap_or_else(|_| panic!("Database is offline"));
    let conn_cache = &cache_pool.get().unwrap_or_else(|_| panic!("Cache is offline"));

    /// Load all categories and make hash set.
    let categories = load_all_categories(conn).unwrap_or_else(|_| panic!("Failed to load categories"));
    update_cache(&categories, "category", conn_cache).unwrap_or_else(|_| panic!("Failed to update categories hash set"));

    println!("Categories loaded");

    /// build list by last reply time desc order for each category. build category meta list with all category ids
    let mut meta_ids = Vec::new();
    for cat in categories.iter() {
        meta_ids.push(cat.id);
        let topic_list = get_topic_list(&cat.id, conn).unwrap_or_else(|_| panic!("Failed to build category lists"));
        build_list(topic_list, &format!("category:{}", &cat.id), conn_cache).unwrap_or_else(|_| panic!("Failed to build category lists"));
    }
    update_meta(meta_ids,"category_id", conn_cache).unwrap_or_else(|_| panic!("Failed to build category meta"));
    println!("Category list and meta data loaded");

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