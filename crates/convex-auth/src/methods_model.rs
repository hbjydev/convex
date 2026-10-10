use convex_core::{Error as AppError, Result as AppResult};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MethodAction {
    AddLocal,
    RemoveLocal,
}
impl MethodAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AddLocal => "add_local",
            Self::RemoveLocal => "remove_local",
        }
    }
    pub fn parse(value: &str) -> AppResult<Self> {
        match value {
            "add_local" => Ok(Self::AddLocal),
            "remove_local" => Ok(Self::RemoveLocal),
            _ => Err(AppError::unauthorized()),
        }
    }
}

#[derive(Clone)]
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
pub struct OidcMethod {
    pub id: Uuid,
    pub issuer: String,
    pub usable: bool,
}

#[derive(Clone)]
pub struct Methods {
    pub local_email: Option<String>,
    pub local_usable: bool,
    pub oidc: Vec<OidcMethod>,
    pub provider_display_name: Option<String>,
    pub can_link_oidc: bool,
    pub can_add_local: bool,
    pub can_remove_local: bool,
    pub pending_action: Option<MethodAction>,
}

#[derive(Clone)]
pub struct LocalProof {
    pub current_password: String,
}

#[derive(Clone)]
pub struct ProveOidc {
    pub action: MethodAction,
}

#[derive(Clone)]
pub struct AddLocal {
    pub email: String,
    pub password: String,
}

#[derive(Clone)]
pub struct RemoveOidc {
    pub identity_id: Uuid,
    pub current_password: String,
}
