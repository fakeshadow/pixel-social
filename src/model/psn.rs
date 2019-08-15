use std::str::FromStr;
use std::string::ToString;

use psn_api_rs::models::{EarnedTrophies, PSNUserTrophySummary};

use crate::model::{common::GetSelfId, errors::ResError};

pub type PSNUser = psn_api_rs::models::PSNUser;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserPSNProfile {
    pub id: u32,
    pub profile: PSNUser,
}

impl Default for UserPSNProfile {
    fn default() -> UserPSNProfile {
        UserPSNProfile {
            id: 0,
            profile: PSNUser {
                online_id: "".to_string(),
                np_id: "".to_string(),
                region: "".to_string(),
                avatar_url: "".to_string(),
                about_me: "".to_string(),
                languages_used: vec![],
                plus: 0,
                trophy_summary: PSNUserTrophySummary {
                    level: 0,
                    progress: 0,
                    earned_trophies: EarnedTrophies {
                        platinum: 0,
                        gold: 0,
                        silver: 0,
                        bronze: 0,
                    },
                },
            },
        }
    }
}

impl GetSelfId for UserPSNProfile {
    fn self_id(&self) -> &u32 {
        &self.id
    }
}

#[derive(Serialize, Deserialize)]
pub struct PSNProfileRequest {
    pub online_id: String,
}


#[derive(Serialize, Deserialize)]
pub struct PSNActivationRequest {
    pub   user_id: Option<u32>,
    pub   online_id: String,
    pub   code: String,
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
    where Self: serde::Serialize {
    fn stringify(&self) -> Result<String, ResError> {
        Ok(serde_json::to_string(&self)?)
    }
}

impl Stringify for PSNProfileRequest {}
impl Stringify for PSNActivationRequest {}
