use chrono::NaiveDateTime;
use tokio_postgres::Row;

use crate::model::{
    category::Category,
    errors::ResError,
    post::Post,
    psn::{UserTrophy, UserTrophySet, UserTrophyTitle},
    talk::{PrivateMessage, PublicMessage, Relation, Talk},
    topic::Topic,
    user::User,
};

pub trait TryFromRow<T>: Sized {
    type Error;
    fn try_from_row(r: &T) -> Result<Self, Self::Error>;
}

impl TryFromRow<Row> for Topic {
    type Error = ResError;
    fn try_from_row(row: &Row) -> Result<Self, Self::Error> {
        Ok(Topic {
            id: row.try_get(0)?,
            user_id: row.try_get(1)?,
            category_id: row.try_get(2)?,
            title: row.try_get(3)?,
            body: row.try_get(4)?,
            thumbnail: row.try_get(5)?,
            created_at: row.try_get(6)?,
            updated_at: row.try_get(7)?,
            is_locked: row.try_get(8)?,
            is_visible: row.try_get(9)?,
            last_reply_time: None,
            reply_count: None,
        })
    }
}

impl TryFromRow<Row> for User {
    type Error = ResError;
    fn try_from_row(row: &Row) -> Result<Self, Self::Error> {
        Ok(User {
            id: row.try_get(0)?,
            username: row.try_get(1)?,
            email: row.try_get(2)?,
            hashed_password: row.try_get(3)?,
            avatar_url: row.try_get(4)?,
            signature: row.try_get(5)?,
            created_at: row.try_get(6)?,
            privilege: row.try_get(7)?,
            show_email: row.try_get(8)?,
            online_status: None,
            last_online: None,
        })
    }
}

impl TryFromRow<Row> for Post {
    type Error = ResError;
    fn try_from_row(row: &Row) -> Result<Self, Self::Error> {
        Ok(Post {
            id: row.try_get(0)?,
            user_id: row.try_get(1)?,
            topic_id: row.try_get(2)?,
            category_id: row.try_get(3)?,
            post_id: row.try_get(4)?,
            post_content: row.try_get(5)?,
            created_at: row.try_get(6)?,
            updated_at: row.try_get(7)?,
            last_reply_time: None,
            is_locked: row.try_get(8)?,
            reply_count: None,
        })
    }
}

impl TryFromRow<Row> for Talk {
    type Error = ResError;
    fn try_from_row(row: &Row) -> Result<Self, Self::Error> {
        Ok(Talk {
            id: row.try_get(0)?,
            name: row.try_get(1)?,
            description: row.try_get(2)?,
            secret: row.try_get(3)?,
            privacy: row.try_get(4)?,
            owner: row.try_get(5)?,
            admin: row.try_get(6)?,
            users: row.try_get(7)?,
        })
    }
}

impl TryFromRow<Row> for Category {
    type Error = ResError;
    fn try_from_row(row: &Row) -> Result<Self, Self::Error> {
        Ok(Category {
            id: row.try_get(0)?,
            name: row.try_get(1)?,
            thumbnail: row.try_get(2)?,
            topic_count: None,
            post_count: None,
            topic_count_new: None,
            post_count_new: None,
        })
    }
}

impl TryFromRow<Row> for Relation {
    type Error = ResError;
    fn try_from_row(row: &Row) -> Result<Self, Self::Error> {
        Ok(Relation {
            friends: row.try_get(0)?,
        })
    }
}

impl TryFromRow<Row> for PublicMessage {
    type Error = ResError;
    fn try_from_row(row: &Row) -> Result<Self, Self::Error> {
        Ok(PublicMessage {
            talk_id: row.try_get(0)?,
            time: row.try_get(1)?,
            text: row.try_get(2)?,
        })
    }
}

impl TryFromRow<Row> for PrivateMessage {
    type Error = ResError;
    fn try_from_row(row: &Row) -> Result<Self, Self::Error> {
        Ok(PrivateMessage {
            user_id: row.try_get(0)?,
            time: row.try_get(2)?,
            text: row.try_get(3)?,
        })
    }
}

impl TryFromRow<Row> for UserTrophyTitle {
    type Error = ResError;
    fn try_from_row(row: &Row) -> Result<Self, Self::Error> {
        Ok(UserTrophyTitle {
            np_id: row.try_get(0)?,
            np_communication_id: row.try_get(1)?,
            is_visible: row.try_get(2)?,
            progress: row.try_get(3)?,
            earned_platinum: row.try_get(4)?,
            earned_gold: row.try_get(5)?,
            earned_silver: row.try_get(6)?,
            earned_bronze: row.try_get(7)?,
            last_update_date: row.try_get(8)?,
        })
    }
}

impl TryFromRow<Row> for UserTrophySet {
    type Error = ResError;
    fn try_from_row(row: &Row) -> Result<Self, Self::Error> {
        let vec = row.try_get(3)?;

        Ok(UserTrophySet {
            np_id: row.try_get(0)?,
            np_communication_id: row.try_get(1)?,
            is_visible: row.try_get(2)?,
            trophies: generate_trophies(vec)?,
        })
    }
}

fn generate_trophies(vec: &str) -> Result<Vec<UserTrophy>, ResError> {
    let len = vec.len();

    let vec: Vec<&str> = if len < 6 {
        Vec::with_capacity(0)
    } else {
        vec[2..(len - 2)].split("\",\"").collect()
    };

    let mut trophies = Vec::with_capacity(vec.len());

    for v in vec.iter() {
        let len = v.len();
        let v: Vec<&str> = v[1..(len - 1)].split(',').collect();
        let earned_date = match v.get(1) {
            Some(s) => {
                let len = s.len();
                if len > 2 {
                    NaiveDateTime::parse_from_str(&s[2..len - 2], "%Y-%m-%d %H:%M:%S").ok()
                } else {
                    None
                }
            }
            None => None,
        };

        let first_earned_date = match v.get(2) {
            Some(s) => {
                let len = s.len();
                if len > 2 {
                    NaiveDateTime::parse_from_str(&s[2..len - 2], "%Y-%m-%d %H:%M:%S").ok()
                } else {
                    None
                }
            }
            None => None,
        };

        trophies.push(UserTrophy {
            trophy_id: v.get(0).ok_or(ResError::PostgresError)?.parse::<u32>()?,
            earned_date,
            first_earned_date,
        })
    }

    Ok(trophies)
}
