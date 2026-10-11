use convex_auth::accounts::models::User;
use convex_ids::UserId;
use convex_workspaces as workspaces;

#[derive(Clone)]
pub struct SetupRequest {
    pub setup_token: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
}

#[derive(Clone)]
pub struct AccountResponse {
    pub user: User,
    pub workspaces: Vec<workspaces::Workspace>,
}

#[derive(Clone)]
pub struct DisableRequest {
    pub user_id: UserId,
    pub disabled: bool,
}

#[derive(Clone)]
pub struct AcceptInvite {
    pub token: String,
    pub email: Option<String>,
    pub password: Option<String>,
    pub display_name: Option<String>,
}
