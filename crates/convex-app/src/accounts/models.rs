use convex_auth::accounts::models::User;
use convex_workspaces as workspaces;
use uuid::Uuid;

#[derive(Clone)]
pub struct SetupRequest {
    pub setup_token: String,
    pub email: String,
    pub password: String,
    pub display_name: String,
    pub workspace_name: String,
    pub workspace_slug: String,
    pub team_name: String,
    pub team_key: String,
}

#[derive(Clone)]
pub struct AccountResponse {
    pub user: User,
    pub workspaces: Vec<workspaces::Workspace>,
}

#[derive(Clone)]
pub struct DisableRequest {
    pub user_id: Uuid,
    pub disabled: bool,
}

#[derive(Clone)]
pub struct AcceptInvite {
    pub token: String,
    pub email: Option<String>,
    pub password: Option<String>,
    pub display_name: Option<String>,
}
