#[derive(Clone)]
pub struct StartRequest {
    pub invitation_token: Option<String>,
}

#[derive(Clone)]
pub struct StartResponse {
    pub authorization_url: String,
}
