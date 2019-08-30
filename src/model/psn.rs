use std::convert::TryFrom;

use chrono::NaiveDateTime;

use crate::model::common::SelfIdString;

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
    pub progress: u32,
    pub earned_platinum: u32,
    pub earned_gold: u32,
    pub earned_silver: u32,
    pub earned_bronze: u32,
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
            progress: u32::from(t.title_detail.progress),
            earned_platinum: e.platinum,
            earned_gold: e.gold,
            earned_silver: e.silver,
            earned_bronze: e.bronze,
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
    pub trophy_id: u32,
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
            trophy_id: u32::from(t.trophy_id),
            earned_date,
            first_earned_date: earned_date,
        }
    }
}

#[derive(Serialize)]
pub struct TrophyData {
    pub trophy_id: u32,
    pub trophy_hidden: bool,
    pub trophy_type: String,
    pub trophy_name: String,
    pub trophy_detail: String,
    pub trophy_icon_url: String,
    pub trophy_rare: u8,
    pub trophy_earned_rate: String,
}

// anti cheating data.
#[derive(Deserialize)]
pub struct PSNTrophyArgumentRequest {
    pub user_id: Option<u32>,
    pub np_communication_id: String,
    pub trophy_id: u32,
    pub should_before: Option<ShouldBeforeAfter>,
    pub should_after: Option<ShouldBeforeAfter>,
    pub should_absent_time: Option<ShouldAbsentTime>,
}

#[derive(Deserialize)]
pub struct ShouldBeforeAfter {
    pub trophy_id: u32,
    pub reason: String,
    pub agreement: Option<u32>,
    pub disagreement: Option<u32>,
}

#[derive(Deserialize)]
pub struct ShouldAbsentTime {
    pub beginning: NaiveDateTime,
    pub ending: Option<NaiveDateTime>,
    pub is_regular: bool,
    pub reason: String,
    pub agreement: Option<u32>,
    pub disagreement: Option<u32>,
}

// general trophy data.
#[derive(Serialize)]
pub struct TrophySetData {
    pub np_communication_id: String,
    pub trophies: Vec<TrophyData>,
}
