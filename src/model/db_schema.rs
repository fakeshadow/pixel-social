use std::convert::TryFrom;

use chrono::NaiveDateTime;
use tokio_postgres::{Row, SimpleQueryRow};

use crate::model::{
    category::Category,
    errors::ResError,
    post::Post,
    psn::UserTrophyTitle,
    talk::{PrivateMessage, PublicMessage, Relation, Talk},
    topic::Topic,
    user::User,
};

impl TryFrom<Row> for User {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(User {
            id: row.try_get(0)?,
            username: row.try_get(1)?,
            email: row.try_get(2)?,
            hashed_password: "1".to_owned(),
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

impl TryFrom<Row> for Topic {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
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

impl TryFrom<Row> for Post {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
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

impl TryFrom<Row> for Talk {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
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

impl TryFrom<Row> for Relation {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(Relation {
            friends: row.try_get(0)?,
        })
    }
}

impl TryFrom<Row> for PublicMessage {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(PublicMessage {
            talk_id: row.try_get(0)?,
            time: row.try_get(1)?,
            text: row.try_get(2)?,
        })
    }
}

impl TryFrom<Row> for PrivateMessage {
    type Error = ResError;
    fn try_from(row: Row) -> Result<Self, Self::Error> {
        Ok(PrivateMessage {
            user_id: row.try_get(0)?,
            time: row.try_get(2)?,
            text: row.try_get(3)?,
        })
    }
}

impl TryFrom<SimpleQueryRow> for Post {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        let post_id = match r.get(4) {
            Some(s) => s.parse::<u32>().ok(),
            None => None,
        };
        Ok(Post {
            id: r
                .get(0)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            user_id: r
                .get(1)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            topic_id: r
                .get(2)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            category_id: r
                .get(3)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            post_id,
            post_content: r.get(5).ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(
                r.get(6).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
            updated_at: NaiveDateTime::parse_from_str(
                r.get(7).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
            last_reply_time: None,
            is_locked: r.get(8) == Some("t"),
            reply_count: None,
        })
    }
}

impl TryFrom<SimpleQueryRow> for Topic {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        Ok(Topic {
            id: r
                .get(0)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            user_id: r
                .get(1)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            category_id: r
                .get(2)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            title: r.get(3).ok_or(ResError::DataBaseReadError)?.to_owned(),
            body: r.get(4).ok_or(ResError::DataBaseReadError)?.to_owned(),
            thumbnail: r.get(5).ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(
                r.get(6).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
            updated_at: NaiveDateTime::parse_from_str(
                r.get(7).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
            is_locked: r.get(8) == Some("t"),
            is_visible: r.get(9) == Some("t"),
            last_reply_time: None,
            reply_count: None,
        })
    }
}

impl TryFrom<SimpleQueryRow> for User {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        Ok(User {
            id: r
                .get(0)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            username: r.get(1).ok_or(ResError::DataBaseReadError)?.to_owned(),
            email: r.get(2).ok_or(ResError::DataBaseReadError)?.to_owned(),
            hashed_password: r.get(3).ok_or(ResError::DataBaseReadError)?.to_owned(),
            avatar_url: r.get(4).ok_or(ResError::DataBaseReadError)?.to_owned(),
            signature: r.get(5).ok_or(ResError::DataBaseReadError)?.to_owned(),
            created_at: NaiveDateTime::parse_from_str(
                r.get(6).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
            privilege: r
                .get(7)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            show_email: r.get(8) == Some("t"),
            online_status: None,
            last_online: None,
        })
    }
}

impl TryFrom<SimpleQueryRow> for Category {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        Ok(Category {
            id: r
                .get(0)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            name: r.get(1).ok_or(ResError::DataBaseReadError)?.to_owned(),
            thumbnail: r.get(2).ok_or(ResError::DataBaseReadError)?.to_owned(),
            topic_count: None,
            post_count: None,
            topic_count_new: None,
            post_count_new: None,
        })
    }
}

impl TryFrom<SimpleQueryRow> for Talk {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        let admin = r.get(6).ok_or(ResError::DataBaseReadError)?;
        let users = r.get(7).ok_or(ResError::DataBaseReadError)?;

        let alen = admin.len();
        let ulen = users.len();

        let admin: Vec<&str> = if alen < 2 {
            Vec::with_capacity(0)
        } else {
            admin[1..(alen - 1)].split(',').collect()
        };
        let users: Vec<&str> = if ulen < 2 {
            Vec::with_capacity(0)
        } else {
            users[1..(ulen - 1)].split(',').collect()
        };

        Ok(Talk {
            id: r
                .get(0)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            name: r.get(1).ok_or(ResError::DataBaseReadError)?.to_owned(),
            description: r.get(2).ok_or(ResError::DataBaseReadError)?.to_owned(),
            secret: r.get(3).ok_or(ResError::DataBaseReadError)?.to_owned(),
            privacy: r
                .get(4)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            owner: r
                .get(5)
                .ok_or(ResError::DataBaseReadError)?
                .parse::<u32>()?,
            admin: admin
                .into_iter()
                .map(|a| a.parse::<u32>())
                .collect::<Result<Vec<u32>, _>>()?,
            users: users
                .into_iter()
                .map(|u| u.parse::<u32>())
                .collect::<Result<Vec<u32>, _>>()?,
        })
    }
}

impl TryFrom<SimpleQueryRow> for UserTrophyTitle {
    type Error = ResError;
    fn try_from(r: SimpleQueryRow) -> Result<Self, Self::Error> {
        Ok(UserTrophyTitle {
            np_id: r.get(0).ok_or(ResError::DataBaseReadError)?.to_owned(),
            np_communication_id: r.get(1).ok_or(ResError::DataBaseReadError)?.to_owned(),
            progress: r.get(2).ok_or(ResError::DataBaseReadError)?.parse::<u8>()?,
            earned_platinum: r.get(3).ok_or(ResError::DataBaseReadError)?.parse::<u8>()?,
            earned_gold: r.get(4).ok_or(ResError::DataBaseReadError)?.parse::<u8>()?,
            earned_silver: r.get(5).ok_or(ResError::DataBaseReadError)?.parse::<u8>()?,
            earned_bronze: r.get(6).ok_or(ResError::DataBaseReadError)?.parse::<u8>()?,
            last_update_date: NaiveDateTime::parse_from_str(
                r.get(7).ok_or(ResError::DataBaseReadError)?,
                "%Y-%m-%d %H:%M:%S%.f",
            )?,
        })
    }
}
