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

        Box::new(Self::simple_query(
            self.db.as_mut().unwrap(),
            query.as_str(),
            self.error_reprot.as_ref().map(|e| e.clone()))
            .map(|_| ()))
    }
}

impl Handler<GetCategories> for DatabaseService {
    type Result = ResponseFuture<Vec<Category>, ResError>;

    fn handle(&mut self, _: GetCategories, _: &mut Self::Context) -> Self::Result {
        Box::new(Self::query_multi_simple_no_limit(
            self.db.as_mut().unwrap(),
            "SELECT * FROM categories",
            self.error_reprot.as_ref().map(|e| e.clone())))
    }
}

impl Handler<AddCategory> for DatabaseService {
    type Result = ResponseActFuture<Self, Category, ResError>;

    fn handle(&mut self, msg: AddCategory, _: &mut Self::Context) -> Self::Result {
        let c = msg.0;

        let query = "SELECT MAX(id) FROM categories";

        let f = Self::query_single_row::<u32>(
            self.db.as_mut().unwrap(),
            query,
            0,
            self.error_reprot.as_ref().map(|e| e.clone()))
            .into_actor(self)
            .and_then(move |cid, act, _| {
                let cid = cid + 1;
                let query = format!("
                    INSERT INTO categories
                    (id, name, thumbnail)
                    VALUES ('{}', '{}', '{}')
                    RETURNING *", cid, c.name.unwrap(), c.thumbnail.unwrap());

                Self::query_one_simple(
                    act.db.as_mut().unwrap(),
                    query.as_str(),
                    act.error_reprot.as_ref().map(|e| e.clone()))
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

        Box::new(Self::query_one_simple(
            self.db.as_mut().unwrap(),
            query.as_str(),
            self.error_reprot.as_ref().map(|e| e.clone()))
            .map(|c| vec![c]))
    }
}