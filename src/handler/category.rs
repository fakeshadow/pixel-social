use std::fmt::Write;
use futures::future::err as ft_err;

use actix::prelude::{
    ActorFuture,
    Future,
    Handler,
    Message,
    ResponseFuture,
    ResponseActFuture,
    WrapFuture,
};

use crate::model::{
    actors::DatabaseService,
    category::{Category, CategoryRequest},
    errors::ResError,
};
use crate::handler::db::{
    SimpleQueryOne,
    SimpleQueryMulti
};


pub struct GetCategories;

pub struct AddCategory(pub CategoryRequest);

pub struct UpdateCategory(pub CategoryRequest);

pub struct RemoveCategory(pub u32);

impl Message for GetCategories {
    type Result = Result<Vec<Category>, ResError>;
}

impl Message for AddCategory {
    type Result = Result<Category, ResError>;
}

impl Message for UpdateCategory {
    type Result = Result<Vec<Category>, ResError>;
}

impl Message for RemoveCategory {
    type Result = Result<(), ResError>;
}

impl Handler<RemoveCategory> for DatabaseService {
    type Result = ResponseFuture<(), ResError>;

    fn handle(&mut self, msg: RemoveCategory, _: &mut Self::Context) -> Self::Result {
        let query = format!("
        DELETE FROM categories
        WHERE id={}", msg.0);

        Box::new(self
            .simple_query_one::<Category>(query.as_str())
            .map(|_| ()))
    }
}

impl Handler<GetCategories> for DatabaseService {
    type Result = ResponseFuture<Vec<Category>, ResError>;

    fn handle(&mut self, _: GetCategories, _: &mut Self::Context) -> Self::Result {
        Box::new(self.simple_query_multi("SELECT * FROM categories", Vec::new()))
    }
}

impl Handler<AddCategory> for DatabaseService {
    type Result = ResponseActFuture<Self, Category, ResError>;

    fn handle(&mut self, msg: AddCategory, _: &mut Self::Context) -> Self::Result {
        let c = msg.0;

        let query = "SELECT MAX(id) FROM categories";

        let f = self
            .simple_query_single_row::<u32>(query, 0)
            .into_actor(self)
            .and_then(move |cid, act, _| {
                let cid = cid + 1;
                let query = format!("
                    INSERT INTO categories
                    (id, name, thumbnail)
                    VALUES ('{}', '{}', '{}')
                    RETURNING *", cid, c.name.unwrap(), c.thumbnail.unwrap());

                act.simple_query_one(query.as_str())
                    .into_actor(act)
            });

        Box::new(f)
    }
}

impl Handler<UpdateCategory> for DatabaseService {
    type Result = ResponseFuture<Vec<Category>, ResError>;

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
            return Box::new(ft_err(ResError::BadRequest));
        };

        Box::new(self.simple_query_one(query.as_str())
            .map(|c| vec![c]))
    }
}