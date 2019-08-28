use std::convert::TryFrom;

use chrono::NaiveDateTime;

use crate::model::{common::SelfIdString, errors::ResError};

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

impl SelfIdString for UserPSNProfile {
    fn self_id_string(&self) -> String {
        self.online_id.to_owned()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserTrophyTitle {
    pub np_id: String,
    pub np_communication_id: String,
    pub is_visible: bool,
    pub progress: u8,
    pub earned_platinum: u8,
    pub earned_gold: u8,
    pub earned_silver: u8,
    pub earned_bronze: u8,
    pub last_update_date: NaiveDateTime,
}

impl TryFrom<TrophyTitleLib> for UserTrophyTitle {
    type Error = ();

    fn try_from(t: TrophyTitleLib) -> Result<Self, Self::Error> {
        let e = &t.title_detail.earned_trophies;

        Ok(UserTrophyTitle {
            np_id: "place_holder".to_string(),
            np_communication_id: t.np_communication_id,
            is_visible: true,
            progress: t.title_detail.progress,
            earned_platinum: e.platinum as u8,
            earned_gold: e.gold as u8,
            earned_silver: e.silver as u8,
            earned_bronze: e.bronze as u8,
            last_update_date: NaiveDateTime::parse_from_str(
                t.title_detail.last_update_date.as_str(),
                "%Y-%m-%dT%H:%M:%S%#z",
            )
            .map_err(|_| ())?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserTrophySet {
    pub np_id: String,
    pub np_communication_id: String,
    pub is_visible: bool,
    pub trophies: Vec<UserTrophy>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserTrophy {
    pub trophy_id: u8,
    pub earned_date: Option<NaiveDateTime>,
    pub first_earned_date: Option<NaiveDateTime>,
}

impl From<&TrophyLib> for UserTrophy {
    fn from(t: &TrophyLib) -> UserTrophy {
        let earned_date = match t.user_info.earned_date.as_ref() {
            Some(t) => NaiveDateTime::parse_from_str(t.as_str(), "%Y-%m-%dT%H:%M:%S%#z").ok(),
            None => None,
        };

        UserTrophy {
            trophy_id: t.trophy_id,
            earned_date,
            first_earned_date: earned_date,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "query_type")]
pub enum PSNRequest {
    Profile {
        online_id: String,
    },
    TrophyTitles {
        online_id: String,
        page: String,
    },
    TrophySet {
        online_id: String,
        np_communication_id: String,
    },
    Auth {
        uuid: Option<String>,
        two_step: Option<String>,
        refresh_token: Option<String>,
    },
    Activation {
        user_id: Option<u32>,
        online_id: String,
        code: String,
    },
}

impl PSNRequest {
    pub fn check_privilege(self, privilege: u32) -> Result<Self, ResError> {
        if privilege < 9 {
            Err(ResError::Unauthorized)
        } else {
            Ok(self)
        }
    }

    pub fn attach_user_id(self, uid: u32) -> Self {
        if let PSNRequest::Activation {
            online_id, code, ..
        } = self
        {
            PSNRequest::Activation {
                user_id: Some(uid),
                online_id,
                code,
            }
        } else {
            panic!("should not happen unless the router code has been changed")
        }
    }
}

//#[derive(Deserialize)]
//pub struct PSNGameMeta {
//    pub
//}
