use actix::prelude::*;
use tokio_postgres::{connect, Client, tls::NoTls, Statement};

pub struct PostgresConnection {
    pub db: Option<Client>,
    pub categories: Option<Statement>,
    pub topics_by_cid: Option<Statement>,
    pub topics_by_id: Option<Statement>,
    pub posts_by_tid: Option<Statement>,
    pub users_by_id: Option<Statement>,
    pub add_topic: Option<Statement>,
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
                        VALUES ($1, $2, $3, $4, $5, $6)
                        RETURNING *")
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