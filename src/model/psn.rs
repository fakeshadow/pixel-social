use std::str::FromStr;
use std::string::ToString;

use crate::model::{
    errors::ResError,
    common::GetSelfId,
};

pub type PSNUser = psn_api_rs::models::PSNUser;

#[derive(Serialize, Deserialize, Debug)]
pub struct UserPSNProfile {
    pub id: u32,
    pub profile: PSNUser,
}


impl GetSelfId for UserPSNProfile {
    fn self_id(&self) -> &u32 {
        &self.id
    }
}


#[derive(Serialize, Deserialize)]
pub struct PSNActivationRequest {
    pub user_id: Option<u32>,
    pub online_id: String,
    pub code: String,
}

impl PSNActivationRequest {
    pub fn attach_user_id(mut self, id: u32) -> Self {
        self.user_id = Some(id);
        self
    }

    pub fn into_request_string(self) -> Result<String, ResError> {
        PSNRequest::Activation(self).stringify()
    }
}


#[derive(Serialize, Deserialize)]
pub struct PSNProfileRequest(pub String);

impl PSNProfileRequest {
    pub fn into_request_string(self) -> Result<String, ResError> {
        PSNRequest::Profile(self.0).stringify()
    }
}


#[derive(Serialize, Deserialize)]
pub enum PSNRequest {
    Activation(PSNActivationRequest),
    Profile(String),
}

impl FromStr for PSNRequest {
    type Err = ResError;
    fn from_str(s: &str) -> Result<PSNRequest, Self::Err> {
        Ok(serde_json::from_str(s)?)
    }
}

impl PSNRequest {
    pub fn stringify(&self) -> Result<String, ResError> {
        Ok(serde_json::to_string(&self)?)
    }
}

