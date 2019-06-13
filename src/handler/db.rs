use std::io;
use futures::{Future, future, IntoFuture};

use actix::prelude::*;
use tokio_postgres::{connect, Client, tls::NoTls, Statement};

use crate::model::{
    errors::ServiceError,
    user::User,
    post::Post,
    category::Category,
    topic::{Topic, TopicRequest},
    common::GlobalGuard,
};

pub struct PostgresConnection {
    db: Option<Client>,
    categories: Option<Statement>,
    topics_by_cid: Option<Statement>,
    topics_by_id: Option<Statement>,
    posts_by_tid: Option<Statement>,
    users_by_id: Option<Statement>,
    add_topic: Option<Statement>,
}

impl Actor for PostgresConnection {
    type Context = Context<Self>;
}

pub type DB = Addr<PostgresConnection>;

impl PostgresConnection {
    pub fn connect(postgres_url: &str) -> Addr<PostgresConnection> {
        let hs = connect(postgres_url, NoTls);

        PostgresConnection::create(move |ctx| {
            let addr = PostgresConnection {
                db: None,
                categories: None,
                topics_by_cid: None,
                topics_by_id: None,
                posts_by_tid: None,
                users_by_id: None,
                add_topic: None,
            };

            hs.map_err(|_| panic!("Can't connect to database"))
                .into_actor(&addr)
                .and_then(|(mut db, conn), addr, ctx| {
                    ctx.wait(
                        db.prepare("SELECT * FROM categories")
                            .map_err(|e| panic!("{}", e))
                            .into_actor(addr)
                            .and_then(|st, addr, _| {
                                addr.categories = Some(st);
                                fut::ok(())
                            })
                    );
                    ctx.wait(
                        db.prepare("SELECT * FROM topics
                        WHERE category_id = ANY($1)
                        ORDER BY last_reply_time DESC
                        OFFSET $2
                        LIMIT 20")
                            .map_err(|e| panic!("{}", e))
                            .into_actor(addr)
                            .and_then(|st, addr, _| {
                                addr.topics_by_cid = Some(st);
                                fut::ok(())
                            })
                    );
                    ctx.wait(
                        db.prepare("SELECT * FROM topics
                        WHERE id = ANY($1)
                        ORDER BY last_reply_time DESC")
                            .map_err(|e| panic!("{}", e))
                            .into_actor(addr)
                            .and_then(|st, addr, _| {
                                addr.topics_by_id = Some(st);
                                fut::ok(())
                            })
                    );
                    ctx.wait(
                        db.prepare("SELECT * FROM posts
                        WHERE topic_id=$1
                        ORDER BY id ASC
                        OFFSET $2
                        LIMIT 20")
                            .map_err(|e| panic!("{}", e))
                            .into_actor(addr)
                            .and_then(|st, addr, _| {
                                addr.posts_by_tid = Some(st);
                                fut::ok(())
                            })
                    );
                    ctx.wait(
                        db.prepare("SELECT * FROM users
                         WHERE id = ANY($1)")
                            .map_err(|e| panic!("{}", e))
                            .into_actor(addr)
                            .and_then(|st, addr, _| {
                                addr.users_by_id = Some(st);
                                fut::ok(())
                            })
                    );
                    ctx.wait(
                        db.prepare("INSERT INTO topics (id, user_id, category_id, thumbnail, title, body)
                        VALUES ($1, $2, $3, $4, $5, $6)")
                            .map_err(|e| panic!("{}", e))
                            .into_actor(addr)
                            .and_then(|st, addr, _| {
                                addr.add_topic = Some(st);
                                fut::ok(())
                            })
                    );

                    addr.db = Some(db);
                    Arbiter::spawn(conn.map_err(|e| panic!("{}", e)));
                    fut::ok(())
                })
                .wait(ctx);
            addr
        })
    }
}


pub struct GetTopics(pub Vec<u32>, pub i64);

impl Message for GetTopics {
    type Result = Result<(Vec<Topic>, Vec<u32>), ServiceError>;
}

impl Handler<GetTopics> for PostgresConnection {
    type Result = ResponseFuture<(Vec<Topic>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetTopics, _: &mut Self::Context) -> Self::Result {
        let topics = Vec::with_capacity(20);
        let ids: Vec<u32> = Vec::with_capacity(20);
        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.topics_by_cid.as_ref().unwrap(), &[&msg.0, &(msg.1 - 1)])
            .from_err()
            .fold((topics, ids), move |(mut topics, mut ids), row| {
                ids.push(row.get(1));
                ids.sort();
                ids.dedup();
                topics.push(Topic {
                    id: row.get(0),
                    user_id: row.get(1),
                    category_id: row.get(2),
                    title: row.get(3),
                    body: row.get(4),
                    thumbnail: row.get(5),
                    created_at: row.get(6),
                    updated_at: row.get(7),
                    last_reply_time: row.get(8),
                    reply_count: row.get(9),
                    is_locked: row.get(10),
                });
                Ok::<_, ServiceError>((topics, ids))
            })
        )
    }
}

pub struct GetCategories;

impl Message for GetCategories {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Handler<GetCategories> for PostgresConnection {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, _: GetCategories, _: &mut Self::Context) -> Self::Result {
        let categories = Vec::new();
        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.categories.as_ref().unwrap(), &[])
            .from_err()
            .fold(categories, move |mut categories, row| {
                categories.push(Category {
                    id: row.get(0),
                    name: row.get(1),
                    topic_count: row.get(2),
                    post_count: row.get(3),
                    subscriber_count: row.get(4),
                    thumbnail: row.get(5),
                });
                Ok::<_, ServiceError>(categories)
            }))
    }
}

pub struct GetTopic(pub u32);

impl Message for GetTopic {
    type Result = Result<Topic, ServiceError>;
}

impl Handler<GetTopic> for PostgresConnection {
    type Result = ResponseFuture<Topic, ServiceError>;

    fn handle(&mut self, msg: GetTopic, _: &mut Self::Context) -> Self::Result {
        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.topics_by_id.as_ref().unwrap(), &[&vec![msg.0]])
            .into_future()
            .map_err(|(e, _)| e)
            .from_err()
            .and_then(|(row, _)| match row {
                Some(row) => Ok(Topic {
                    id: row.get(0),
                    user_id: row.get(1),
                    category_id: row.get(2),
                    title: row.get(3),
                    body: row.get(4),
                    thumbnail: row.get(5),
                    created_at: row.get(6),
                    updated_at: row.get(7),
                    last_reply_time: row.get(8),
                    reply_count: row.get(9),
                    is_locked: row.get(10),
                }),
                None => Err(ServiceError::InternalServerError)
            })
        )
    }
}

pub struct GetPosts(pub u32, pub i64);

impl Message for GetPosts {
    type Result = Result<(Vec<Post>, Vec<u32>), ServiceError>;
}

impl Handler<GetPosts> for PostgresConnection {
    type Result = ResponseFuture<(Vec<Post>, Vec<u32>), ServiceError>;

    fn handle(&mut self, msg: GetPosts, _: &mut Self::Context) -> Self::Result {
        let posts = Vec::with_capacity(20);
        let ids: Vec<u32> = Vec::with_capacity(20);
        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.posts_by_tid.as_ref().unwrap(), &[&msg.0, &(msg.1 - 1)])
            .from_err()
            .fold((posts, ids), move |(mut posts, mut ids), row| {
                ids.push(row.get(1));
                posts.push(Post {
                    id: row.get(0),
                    user_id: row.get(1),
                    topic_id: row.get(2),
                    post_id: row.get(3),
                    post_content: row.get(4),
                    created_at: row.get(5),
                    updated_at: row.get(6),
                    last_reply_time: row.get(7),
                    reply_count: row.get(8),
                    is_locked: row.get(9),
                });
                Ok::<_, ServiceError>((posts, ids))
            })
            .map(|(p, mut i)| {
                i.sort();
                i.dedup();
                (p, i)
            })
        )
    }
}


pub struct GetUsers(pub Vec<u32>);

impl Message for GetUsers {
    type Result = Result<Vec<User>, ServiceError>;
}

impl Handler<GetUsers> for PostgresConnection {
    type Result = ResponseFuture<Vec<User>, ServiceError>;

    fn handle(&mut self, msg: GetUsers, _: &mut Self::Context) -> Self::Result {
        let users = Vec::with_capacity(21);

        Box::new(self.db
            .as_mut()
            .unwrap()
            .query(self.users_by_id.as_ref().unwrap(), &[&msg.0])
            .from_err()
            .fold(users, move |mut users, row| {
                users.push(User {
                    id: row.get(0),
                    username: row.get(1),
                    email: row.get(2),
                    hashed_password: "1".to_owned(),
                    avatar_url: row.get(4),
                    signature: row.get(5),
                    created_at: row.get(6),
                    updated_at: row.get(7),
                    is_admin: row.get(8),
                    blocked: row.get(9),
                    show_email: row.get(10),
                    show_created_at: row.get(11),
                    show_updated_at: row.get(12),
                });
                Ok::<_, ServiceError>(users)
            })
        )
    }
}

pub struct AddTopic(pub TopicRequest, pub GlobalGuard);

impl Message for AddTopic {
    type Result = Result<(), ServiceError>;
}

impl Handler<AddTopic> for PostgresConnection {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: AddTopic, _: &mut Self::Context) -> Self::Result {
        let id = match msg.1.lock() {
            Ok(mut var) => var.next_tid(),
            Err(_) => return Box::new(future::err(ServiceError::InternalServerError))
        };
        let t = match msg.0.make_topic(&id) {
            Ok(t) => t,
            Err(e) => return Box::new(future::err(e))
        };

        Box::new(
            self.db
                .as_mut()
                .unwrap()
                .query(self.add_topic.as_ref().unwrap(),
                       &[&id, &t.user_id, &t.category_id, &t.thumbnail, &t.title, &t.body])
                .into_future()
                .map_err(|(e, _)| e)
                .from_err()
                .and_then(|(t, _)| {
                    Ok(())
                })
        )
    }
}