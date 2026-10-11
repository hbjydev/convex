use crate::{
    accounts::models::*,
    control_plane::{ControlPlane, finish},
};
use convex_auth::{
    accounts::models::*,
    credentials::{Credentials, SessionUpdate},
    security as sec,
};
use convex_core::{Error as AppError, Result as AppResult};

#[cfg(test)]
mod tests;

pub async fn setup(application: ControlPlane, req: SetupRequest) -> AppResult<SessionUpdate<User>> {
    application.rate_limiter.check("setup").await?;
    let expected = application
        .security
        .setup_token
        .as_ref()
        .ok_or_else(AppError::forbidden)?;
    // Fixed-length digest comparison avoids leaking setup credential prefixes.
    if sec::digest(expected)
        .bytes()
        .zip(sec::digest(&req.setup_token).bytes())
        .fold(0u8, |a, (b, c)| a | (b ^ c))
        != 0
    {
        return Err(AppError::forbidden());
    }
    let email = sec::email(&req.email)?;
    let display_name = sec::name(&req.display_name)?;
    let hash = sec::hash_password(req.password).await?;
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let initialized = work.installation().initialized().await?;
        if initialized {
            return Err(AppError::conflict("This instance has already been set up."));
        }
        let user = work
            .accounts()
            .create_local(email, hash, display_name, true)
            .await?;
        work.installation().mark_initialized().await?;
        let token = work
            .sessions()
            .new_session(user.id, sec::LoginMethod::Local)
            .await?;
        Ok(SessionUpdate::new(&token, user))
    }
    .await;
    finish(work, result).await
}

pub async fn me(application: ControlPlane, credentials: Credentials) -> AppResult<AccountResponse> {
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let user = work.accounts().authenticate(credentials.clone()).await?;
        let workspaces = work.workspaces().list_for_user(user.id).await?;
        Ok(AccountResponse { user, workspaces })
    }
    .await;
    finish(work, result).await
}

pub async fn disable(
    application: ControlPlane,
    credentials: Credentials,
    req: DisableRequest,
) -> AppResult<MessageResponse> {
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let actor = work.accounts().authenticate(credentials.clone()).await?;
        if !actor.is_instance_admin {
            return Err(AppError::forbidden());
        }
        if req.user_id == actor.id && req.disabled {
            return Err(AppError::conflict("You cannot disable your own account."));
        }
        // Team operations lock workspace rows too; an account cannot lose its final active ownership.
        let spaces = work.workspaces().lock_all().await?;
        let target = work.accounts().find_for_update(req.user_id).await?;
        if req.disabled {
            if target.is_instance_admin {
                work.accounts()
                    .protect_enabled_administrators(target.id)
                    .await?;
            }
            for space in spaces {
                work.memberships()
                    .protect_last_owner(space, target.id)
                    .await?;
            }
        }
        work.accounts()
            .set_disabled(target.id, req.disabled)
            .await?;
        work.sessions().revoke_all(target.id).await?;
        Ok(MessageResponse {
            message: if req.disabled {
                "Account disabled. All sessions revoked."
            } else {
                "Account enabled. Sign in again to continue."
            }
            .into(),
        })
    }
    .await;
    finish(work, result).await
}
