use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Clone)]
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
pub struct User {
    pub id: Uuid,
    pub display_name: String,
    pub is_instance_admin: bool,
}

#[derive(Clone)]
pub struct BootstrapInfo {
    pub setup_required: bool,
    pub setup_enabled: bool,
    pub mail_available: bool,
    pub oidc_display_name: Option<String>,
    pub oidc_required: bool,
}

#[derive(Clone)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Clone)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Clone)]
pub struct PasswordChange {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Clone)]
pub struct RecoveryRequest {
    pub email: String,
}

#[derive(Clone)]
pub struct ResetRequest {
    pub token: String,
    pub password: String,
}

#[derive(Clone)]
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
pub struct SessionInfo {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_activity_at: DateTime<Utc>,
    pub current: bool,
}

#[derive(Clone)]
pub struct RevokeSession {
    pub session_id: Uuid,
}

#[derive(Clone)]
#[cfg_attr(feature = "postgres", derive(sqlx::FromRow))]
pub struct AdminAccount {
    pub id: Uuid,
    pub display_name: String,
    pub email: Option<String>,
    pub disabled: bool,
    pub is_instance_admin: bool,
}
