use std::str::FromStr;

use chrono::NaiveDateTime;

use crate::model::{
    common::GetSelfId,
    errors::ResError,
};
use crate::model::user::User;

pub type TrophyTitleLib = psn_api_rs::models::TrophyTitle;
pub type PSNUserLib = psn_api_rs::models::PSNUser;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserPSNProfile {
    pub id: Option<u32>,
    pub online_id: String,
    pub np_id: String,
    pub region: String,
    pub avatar_url: String,
    pub about_me: String,
    pub languages_used: Vec<String>,
    pub plus: u8,
    pub level: u8,
    pub progress: u8,
    pub platinum: u32,
    pub gold: u32,
    pub silver: u32,
    pub bronze: u32,
}

impl Default for UserPSNProfile {
    fn default() -> UserPSNProfile {
        UserPSNProfile {
            id: None,
            online_id: "".to_string(),
            np_id: "".to_string(),
            region: "".to_string(),
            avatar_url: "".to_string(),
            about_me: "".to_string(),
            languages_used: vec![],
            plus: 0,
            level: 0,
            progress: 0,
            platinum: 0,
            gold: 0,
            silver: 0,
            bronze: 0,
        }
    }
}

impl From<PSNUserLib> for UserPSNProfile {
    fn from(u: PSNUserLib) -> UserPSNProfile {
        UserPSNProfile {
            id: None,
            online_id: u.online_id,
            np_id: u.np_id,
            region: u.region,
            avatar_url: u.avatar_url,
            about_me: u.about_me,
            languages_used: u.languages_used,
            plus: u.plus,
            level: u.trophy_summary.level,
            progress: u.trophy_summary.progress,
            platinum: u.trophy_summary.earned_trophies.platinum,
            gold: u.trophy_summary.earned_trophies.gold,
            silver: u.trophy_summary.earned_trophies.silver,
            bronze: u.trophy_summary.earned_trophies.bronze,
        }
    }
}


impl GetSelfId for UserPSNProfile {
    fn self_id(&self) -> u32 {
        self.id.unwrap_or(0)
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct UserTrophyTitles {
    pub id: u32,
    pub online_id: String,
    pub np_id: String,
    pub titles: Vec<TrophyTitleLib>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserTrophyTitle {
    pub np_id: String,
    pub np_communication_id: String,
    pub progress: u8,
    pub earned_trophies: Vec<u32>,
    // psn last update time
    pub last_update_date: NaiveDateTime,
    // self db last update time
    pub last_update_time: NaiveDateTime,
}


#[derive(Serialize, Deserialize)]
pub struct PSNAuthRequest {
    pub uuid: String,
    pub two_step: String,
}

impl Default for PSNAuthRequest {
    fn default() -> PSNAuthRequest {
        PSNAuthRequest {
            uuid: "".to_string(),
            two_step: "".to_string(),
        }
    }
}

impl PSNAuthRequest {
    pub fn check_privilege(self, privilege: u32) -> Result<Self, ResError> {
        if privilege < 9 {
            Err(ResError::Unauthorized)
        } else {
            Ok(self)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PSNProfileRequest {
    pub online_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct PSNTrophyRequest {
    pub online_id: String,
    pub page: Option<i64>,
    pub np_communication_id: Option<String>,
}


#[derive(Serialize, Deserialize)]
pub struct PSNActivationRequest {
    pub user_id: Option<u32>,
    pub online_id: String,
    pub code: String,
}

impl PSNActivationRequest {
    pub fn attach_user_id(mut self, uid: u32) -> Self {
        self.user_id = Some(uid);
        self
    }
}

impl FromStr for PSNActivationRequest {
    type Err = ResError;
    fn from_str(s: &str) -> Result<PSNActivationRequest, Self::Err> {
        Ok(serde_json::from_str(s)?)
    }
}

pub trait Stringify
    where
        Self: serde::Serialize,
{
    fn stringify(&self) -> Result<String, ResError> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Stringify for PSNAuthRequest {}

impl Stringify for PSNProfileRequest {}

impl Stringify for PSNTrophyRequest {}

impl Stringify for PSNActivationRequest {}
