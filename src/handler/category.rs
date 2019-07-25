use std::fmt::Write;
use futures::future::err as ft_err;

use actix::prelude::{
    ActorFuture,
    Future,
    Handler,
    Message,
    ResponseFuture,
    ResponseActFuture,
    WrapFuture
};

use crate::model::{
    actors::DatabaseService,
    category::{Category, CategoryRequest},
    errors::ServiceError,
};
use crate::handler::db;

pub struct GetCategories;

pub struct AddCategory(pub CategoryRequest);

pub struct UpdateCategory(pub CategoryRequest);

pub struct RemoveCategory(pub u32);

impl Message for GetCategories {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Message for AddCategory {
    type Result = Result<Category, ServiceError>;
}

impl Message for UpdateCategory {
    type Result = Result<Vec<Category>, ServiceError>;
}

impl Message for RemoveCategory {
    type Result = Result<(), ServiceError>;
}

impl Handler<RemoveCategory> for DatabaseService {
    type Result = ResponseFuture<(), ServiceError>;

    fn handle(&mut self, msg: RemoveCategory, _: &mut Self::Context) -> Self::Result {
        let query = format!("
        DELETE FROM categories
        WHERE id={}", msg.0);

        Box::new(db::simple_query(self.db.as_mut().unwrap(), &query).map(|_| ()))
    }
}

impl Handler<GetCategories> for DatabaseService {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, _: GetCategories, _: &mut Self::Context) -> Self::Result {
        let query = "SELECT * FROM categories";
        Box::new(db::query_all_simple(self.db.as_mut().unwrap(), query))
    }
}

impl Handler<AddCategory> for DatabaseService {
    type Result = ResponseActFuture<Self, Category, ServiceError>;

    fn handle(&mut self, msg: AddCategory, _: &mut Self::Context) -> Self::Result {
        let c = msg.0;

        let query = "SELECT MAX(id) FROM categories";

        let f = db::query_single_row::<u32>(self.db.as_mut().unwrap(), query, 0)
            .into_actor(self)
            .and_then(move |cid, addr, _| {
                let cid = cid + 1;
                let query = format!("
                    INSERT INTO categories
                    (id, name, thumbnail)
                    VALUES ('{}', '{}', '{}')
                    RETURNING *", cid, c.name.unwrap(), c.thumbnail.unwrap());

                db::query_one_simple(addr.db.as_mut().unwrap(), &query)
                    .into_actor(addr)
            });

        Box::new(f)
    }
}

impl Handler<UpdateCategory> for DatabaseService {
    type Result = ResponseFuture<Vec<Category>, ServiceError>;

    fn handle(&mut self, msg: UpdateCategory, _: &mut Self::Context) -> Self::Result {
        let c = msg.0;

        let mut query = String::new();
        query.push_str("UPDATE categories SET");
        if let Some(s) = c.thumbnail {
            let _ = write!(&mut query, " thumbnail='{}',", s);
        }
        if let Some(s) = c.name {
            let _ = write!(&mut query, " name='{}',", s);
        }
        if query.ends_with(",") {
            query.remove(query.len() - 1);
            let _ = write!(&mut query, " WHERE id='{}' RETURNING *", c.id.unwrap());
        } else {
            return Box::new(ft_err(ServiceError::BadRequest));
        };

        Box::new(db::query_one_simple(self.db.as_mut().unwrap(), query.as_str())
            .map(|c| vec![c]))
    }
}