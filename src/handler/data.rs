use core::ops::Deref;

use std::rc::Rc;

use actix_web::{
    dev::{Payload, PayloadStream},
    FromRequest, HttpRequest,
};
use futures::future::{ready, Ready};

use crate::model::errors::ResError;

pub struct DataRc<T>(Rc<T>);

impl<T> DataRc<T> {
    pub fn new(t: T) -> Self {
        Self(Rc::new(t))
    }

    pub fn get_ref(&self) -> &T {
        &*self
    }
}

impl<T> Clone for DataRc<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Deref for DataRc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> FromRequest for DataRc<T>
where
    T: 'static,
{
    type Error = ResError;
    type Future = Ready<Result<Self, Self::Error>>;
    type Config = ();

    fn from_request(req: &HttpRequest, _: &mut Payload<PayloadStream>) -> Self::Future {
        ready(
            req.app_data::<DataRc<T>>()
                .cloned()
                .ok_or(ResError::InternalServerError),
        )
    }
}
