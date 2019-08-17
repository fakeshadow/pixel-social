use std::str::FromStr;

use chrono::NaiveDateTime;

use crate::model::user::User;
use crate::model::{common::GetSelfId, errors::ResError};
use serde::export::TryFrom;

pub type TrophyTitleLib = psn_api_rs::models::TrophyTitle;
pub type TrophyTitlesLib = psn_api_rs::models::TrophyTitles;
pub type PSNUserLib = psn_api_rs::models::PSNUser;
pub type TrophySetLib = psn_api_rs::models::TrophySet;
pub type TrophyLib = psn_api_rs::models::Trophy;

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
pub struct UserTrophyTitle {
    pub np_id: String,
    pub np_communication_id: String,
    pub progress: u8,
    pub earned_platinum: u8,
    pub earned_gold: u8,
    pub earned_silver: u8,
    pub earned_bronze: u8,
    // psn last update time
    pub last_update_date: NaiveDateTime,
}

impl TryFrom<TrophyTitleLib> for UserTrophyTitle {
    type Error = ();

    fn try_from(t: TrophyTitleLib) -> Result<Self, Self::Error> {
        let e = &t.title_detail.earned_trophies;

        Ok(UserTrophyTitle {
            np_id: "place_holder".to_string(),
            np_communication_id: t.np_communication_id,
            progress: t.title_detail.progress,
            earned_platinum: e.platinum as u8,
            earned_gold: e.gold as u8,
            earned_silver: e.silver as u8,
            earned_bronze: e.bronze as u8,
            last_update_date: NaiveDateTime::parse_from_str(
                t.title_detail.last_update_date.as_str(),
                "%Y-%m-%d %H:%M:%S%.f",
            )
            .map_err(|_| ())?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserTrophySet {
    pub id: u32,
    pub online_id: String,
    pub np_id: String,
    pub titles: Vec<UserTrophy>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserTrophy {
    pub trophy_id: u8,
    pub earned_date: Option<NaiveDateTime>,
}

impl From<&TrophyLib> for UserTrophy {
    fn from(t: &TrophyLib) -> UserTrophy {
        UserTrophy {
            trophy_id: t.trophy_id,
            earned_date: t
                .trophy_detail
                .as_ref()
                .map(|t| NaiveDateTime::parse_from_str(t.as_str(), "%Y-%m-%d %H:%M:%S%.f").ok())
                .unwrap_or(None),
        }
    }
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
