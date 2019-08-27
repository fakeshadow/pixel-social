use crate::handler::db::DatabaseService;
use crate::model::{
    category::{Category, CategoryRequest},
    common::GlobalVars,
    errors::ResError,
    post::{Post, PostRequest},
    topic::{Topic, TopicRequest},
    user::UpdateRequest,
};

impl DatabaseService {
    pub async fn admin_update_topic(
        &self,
        self_level: u32,
        t: &TopicRequest,
    ) -> Result<Topic, ResError> {
        let _ = update_topic_check(self_level, &t)?;
        self.update_topic(t).await
    }

    pub async fn admin_update_post(
        &self,
        self_level: u32,
        p: PostRequest,
    ) -> Result<Post, ResError> {
        let _ = update_post_check(self_level, &p)?;
        self.update_post(p).await
    }

    pub async fn admin_add_category(
        &self,
        self_level: u32,
        req: CategoryRequest,
        g: &GlobalVars,
    ) -> Result<Category, ResError> {
        let _ = update_category_check(self_level, &req)?;
        self.add_category(req, g).await
    }

    pub async fn admin_update_category(
        &self,
        self_level: u32,
        req: CategoryRequest,
    ) -> Result<Category, ResError> {
        let _ = update_category_check(self_level, &req)?;
        self.update_category(req).await
    }

    pub async fn admin_remove_category(
        &self,
        cid: u32,
        self_level: u32,
    ) -> Result<(), ResError> {
        let _ = check_admin_level(&Some(1), self_level, 9)?;
        self.remove_category(cid).await
    }

    pub async fn update_user_check(
        &self,
        self_level: u32,
        u: UpdateRequest,
    ) -> Result<UpdateRequest, ResError> {
        let id = vec![u.id.as_ref().copied().unwrap_or(0)];

        let user = self.get_users_by_id(&id).await?;
        let user = user.first().ok_or(ResError::BadRequest)?;

        if self_level <= user.privilege { return Err(ResError::Unauthorized); }

        check_admin_level(&u.privilege, self_level, 9).map(|_| u)
    }
}

type QueryResult = Result<(), ResError>;

fn update_category_check(lv: u32, req: &CategoryRequest) -> QueryResult {
    check_admin_level(&req.name, lv, 3)?;
    check_admin_level(&req.thumbnail, lv, 3)
}

fn update_topic_check(lv: u32, req: &TopicRequest) -> QueryResult {
    check_admin_level(&req.title, lv, 3)?;
    check_admin_level(&req.body, lv, 3)?;
    check_admin_level(&req.thumbnail, lv, 3)?;
    check_admin_level(&req.is_locked, lv, 2)
}

fn update_post_check(lv: u32, req: &PostRequest) -> QueryResult {
    check_admin_level(&req.topic_id, lv, 3)?;
    check_admin_level(&req.post_id, lv, 3)?;
    check_admin_level(&req.post_content, lv, 3)?;
    check_admin_level(&req.is_locked, lv, 2)
}

fn check_admin_level<T: Sized>(
    t: &Option<T>,
    self_admin_level: u32,
    baseline_admin_level: u32,
) -> Result<(), ResError> {
    if let Some(_value) = t {
        if self_admin_level < baseline_admin_level {
            return Err(ResError::Unauthorized);
        }
    }
    Ok(())
}
