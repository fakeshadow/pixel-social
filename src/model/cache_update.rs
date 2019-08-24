use crate::model::{post::Post, topic::Topic, user::User};

pub enum FailedType {
    Update,
    New,
}

pub struct FailedCollection {
    pub topic: Vec<(Topic, FailedType)>,
    pub post: Vec<(Post, FailedType)>,
    pub user: Vec<(User, FailedType)>,
}

impl FailedCollection {
    pub fn add_topic_new(&mut self, t: Topic) {
        self.topic.push((t, FailedType::New));
    }

    pub fn add_post_new(&mut self, t: Post) {
        self.post.push((t, FailedType::New));
    }

    pub fn add_topic_update(&mut self, t: Vec<Topic>) {
        for t in t.into_iter() {
            self.topic.push((t, FailedType::Update));
        }
    }

    pub fn add_post_update(&mut self, p: Vec<Post>) {
        for p in p.into_iter() {
            self.post.push((p, FailedType::Update));
        }
    }

    pub fn add_user(&mut self, u: Vec<User>) {
        for u in u.into_iter() {
            self.user.push((u, FailedType::Update));
        }
    }

    pub fn remove_by_uids(&mut self, ids: &[u32]) {
        for id in ids.iter() {
            for (index, (u, _)) in self.user.iter().enumerate() {
                if id == &u.id {
                    self.user.remove(index);
                    break;
                }
            }
        }
    }

    pub fn remove_by_pids(&mut self, ids: &[u32]) {
        for id in ids.iter() {
            for (index, (p, _)) in self.post.iter().enumerate() {
                if id == &p.id {
                    self.post.remove(index);
                    break;
                }
            }
        }
    }

    pub fn remove_by_tids(&mut self, ids: &[u32]) {
        for id in ids.iter() {
            for (index, (t, _)) in self.topic.iter().enumerate() {
                if id == &t.id {
                    self.topic.remove(index);
                    break;
                }
            }
        }
    }
}

impl Default for FailedCollection {
    fn default() -> FailedCollection {
        FailedCollection {
            topic: vec![],
            post: vec![],
            user: vec![],
        }
    }
}
