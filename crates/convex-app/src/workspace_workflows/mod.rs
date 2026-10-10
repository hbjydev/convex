use crate::{
    accounts::models::AcceptInvite,
    control_plane::{ControlPlane, finish},
};
use convex_auth::{
    accounts::models::MessageResponse,
    credentials::{Credentials, SessionUpdate},
    security as sec,
};
use convex_core::{Error as AppError, Result as AppResult};
use convex_workspaces::{
    validation::{self, manage, role, workspace_id},
    workspaces_model::*,
};
use uuid::Uuid;

#[cfg(test)]
mod tests;

pub async fn create(
    application: ControlPlane,
    credentials: Credentials,
    req: CreateWorkspace,
) -> AppResult<Workspace> {
    let name = sec::name(&req.name)?;
    let slug = validation::workspace_slug(&req.slug)?;
    let team_name = sec::name(&req.team_name)?;
    let team_key = validation::team_key(&req.team_key)?;
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let actor = work.accounts().authenticate(credentials.clone()).await?;
        if !actor.is_instance_admin {
            return Err(AppError::forbidden());
        }
        let id = work
            .workspaces()
            .create(actor.id, name.clone(), slug.clone(), team_name, team_key)
            .await?;
        Ok(Workspace {
            id,
            name,
            slug,
            role: "owner".into(),
        })
    }
    .await;
    finish(work, result).await
}

pub async fn detail(
    application: ControlPlane,
    credentials: Credentials,
    space: String,
) -> AppResult<WorkspaceDetail> {
    let space = workspace_id(&space)?;
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let user = work.accounts().authenticate(credentials.clone()).await?;
        let (_, role) = work.memberships().for_user(space, user.id).await?;
        let (name, slug) = work.workspaces().identity(space).await?;
        let members = work.memberships().list(space).await?;
        let rows = work.teams().list(space).await?;
        let mut teams = Vec::new();
        for (id, name, key) in rows {
            let statuses = work.workflows().for_team(id).await?;
            teams.push(Team {
                id,
                name,
                key,
                statuses,
            });
        }
        Ok(WorkspaceDetail {
            workspace: Workspace {
                id: space,
                name,
                slug,
                role,
            },
            members,
            teams,
        })
    }
    .await;
    finish(work, result).await
}

pub async fn rename(
    application: ControlPlane,
    credentials: Credentials,
    space: String,
    req: RenameRequest,
) -> AppResult<MessageResponse> {
    let space = workspace_id(&space)?;
    let name = sec::name(&req.name)?;
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let user = work.accounts().authenticate(credentials.clone()).await?;
        manage(&work.memberships().for_user(space, user.id).await?.1)?;
        work.workspaces().rename(space, name).await?;
        Ok(MessageResponse {
            message: "Workspace renamed.".into(),
        })
    }
    .await;
    finish(work, result).await
}

pub async fn team(
    application: ControlPlane,
    credentials: Credentials,
    space: String,
    req: TeamRequest,
) -> AppResult<MessageResponse> {
    let space = workspace_id(&space)?;
    let name = sec::name(&req.name)?;
    let key = validation::team_key(&req.key)?;
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let user = work.accounts().authenticate(credentials.clone()).await?;
        manage(&work.memberships().for_user(space, user.id).await?.1)?;
        work.teams().create(space, name, key).await?;
        Ok(MessageResponse {
            message: "Team created with its initial workflow.".into(),
        })
    }
    .await;
    finish(work, result).await
}

pub async fn rename_team(
    application: ControlPlane,
    credentials: Credentials,
    space: String,
    team: String,
    req: RenameRequest,
) -> AppResult<MessageResponse> {
    let space = workspace_id(&space)?;
    let team = workspace_id(&team)?;
    let name = sec::name(&req.name)?;
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let user = work.accounts().authenticate(credentials.clone()).await?;
        manage(&work.memberships().for_user(space, user.id).await?.1)?;
        work.teams().rename(space, team, name).await?;
        Ok(MessageResponse {
            message: "Team renamed. Its key stays the same.".into(),
        })
    }
    .await;
    finish(work, result).await
}

pub async fn change_member(
    application: ControlPlane,
    credentials: Credentials,
    space: String,
    req: MembershipChange,
) -> AppResult<MessageResponse> {
    let space = workspace_id(&space)?;
    if req.remove == req.role.is_some() {
        return Err(AppError::validation(
            "Choose either a role change or membership removal.",
        ));
    }
    if let Some(value) = &req.role {
        role(value, true)?;
    }
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let actor = work.accounts().authenticate(credentials.clone()).await?;
        let (actor_membership, actor_role) = work.memberships().for_user(space, actor.id).await?;
        manage(&actor_role)?;
        let (user, target_role) = work.memberships().target(space, req.membership_id).await?;
        // Admins manage ordinary members; only owners grant or change administration.
        if actor_role != "owner"
            && (target_role != "member" || req.role.as_deref().is_some_and(|r| r != "member"))
        {
            return Err(AppError::forbidden());
        }
        if req.remove || req.role.as_deref() != Some("owner") {
            work.memberships().protect_last_owner(space, user).await?;
        }
        if req.remove {
            work.issues()
                .clear_assignments(space, req.membership_id, actor_membership)
                .await?;
            work.memberships().remove(req.membership_id).await?;
        } else {
            work.memberships()
                .change_role(req.membership_id, req.role.expect("role validated above"))
                .await?;
        }
        work.sessions().revoke_all(user).await?;
        Ok(MessageResponse {
            message: "Membership updated. This person's sessions were revoked.".into(),
        })
    }
    .await;
    finish(work, result).await
}

pub async fn invite(
    application: ControlPlane,
    credentials: Credentials,
    space: String,
    req: InviteRequest,
) -> AppResult<InviteLink> {
    let space = workspace_id(&space)?;
    role(&req.role, false)?;
    let to = req.deliver_to.as_deref().map(sec::email).transpose()?;
    if to.is_some() && application.security.mail.is_none() {
        return Err(AppError::validation(
            "Email delivery is unavailable. Create a link to share directly.",
        ));
    }
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let actor = work.accounts().authenticate(credentials.clone()).await?;
        let (membership, actor_role) = work.memberships().for_user(space, actor.id).await?;
        manage(&actor_role)?;
        if req.role == "admin" && actor_role != "owner" {
            return Err(AppError::forbidden());
        }
        let token = sec::token();
        let id = Uuid::new_v4();
        let expires_at = work
            .invitations()
            .create(id, space, membership, req.role, sec::digest(&token))
            .await?;
        Ok((id, token, expires_at))
    }
    .await;
    let (id, token, expires_at) = finish(work, result).await?;
    let url = format!("{}/invite#{}", application.security.origin, token);
    let delivered = if let Some(to) = to {
        let message = format!(
            "Join using this single-use invitation within 7 days:\n{url}\nOnly share this link with the intended teammate."
        );
        match application
            .security
            .send(&to, "Join your workspace on Convex", message)
            .await
        {
            Ok(sent) => sent,
            Err(error) => {
                tracing::error!(%error, "Invitation delivery failed; administrator can share the returned link");
                false
            }
        }
    } else {
        false
    };
    Ok(InviteLink {
        id,
        url,
        expires_at,
        email_delivered: delivered,
    })
}

pub async fn invitations(
    application: ControlPlane,
    credentials: Credentials,
    space: String,
) -> AppResult<Vec<Invitation>> {
    let space = workspace_id(&space)?;
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let actor = work.accounts().authenticate(credentials.clone()).await?;
        manage(&work.memberships().for_user(space, actor.id).await?.1)?;
        let invitations = work.invitations().list(space).await?;
        Ok(invitations)
    }
    .await;
    finish(work, result).await
}

pub async fn revoke_invitation(
    application: ControlPlane,
    credentials: Credentials,
    space: String,
    req: RevokeInvitation,
) -> AppResult<MessageResponse> {
    let space = workspace_id(&space)?;
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let actor = work.accounts().authenticate(credentials.clone()).await?;
        manage(&work.memberships().for_user(space, actor.id).await?.1)?;
        work.invitations().revoke(space, req.invitation_id).await?;
        Ok(MessageResponse {
            message: "Invitation revoked.".into(),
        })
    }
    .await;
    finish(work, result).await
}

pub async fn preview(application: ControlPlane, req: InviteToken) -> AppResult<InvitePreview> {
    application
        .rate_limiter
        .check(&format!("invitation-preview:{}", sec::digest(&req.token)))
        .await?;
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let invitation = work.invitations().valid_token(req.token.clone()).await?;
        Ok(InvitePreview {
            workspace_name: invitation.workspace_name,
            role: invitation.role,
        })
    }
    .await;
    finish(work, result).await
}

pub async fn accept(
    application: ControlPlane,
    credentials: Credentials,
    req: AcceptInvite,
) -> AppResult<SessionUpdate<Workspace>> {
    application
        .rate_limiter
        .check(&format!("invitation-accept:{}", sec::digest(&req.token)))
        .await?;
    let local_credentials =
        if req.email.is_none() && req.password.is_none() && req.display_name.is_none() {
            None
        } else {
            Some((
                sec::email(
                    req.email
                        .as_deref()
                        .ok_or_else(|| AppError::validation("Enter your email."))?,
                )?,
                sec::hash_password(
                    req.password
                        .ok_or_else(|| AppError::validation("Enter your password."))?,
                )
                .await?,
                sec::name(
                    req.display_name
                        .as_deref()
                        .ok_or_else(|| AppError::validation("Enter your name."))?,
                )?,
            ))
        };
    let mut work = application.transactions.begin_administration().await?;
    let result = async {
        let invitation = work.invitations().valid_token(req.token.clone()).await?;
        let new_local = local_credentials.is_some();
        let user = if let Some((email, hash, name)) = local_credentials {
            work.accounts()
                .create_local(email, hash, name, false)
                .await?
        } else {
            work.accounts().authenticate(credentials.clone()).await?
        };
        let method = if new_local {
            sec::LoginMethod::Local
        } else {
            work.sessions().session_method(credentials.clone()).await?
        };
        work.invitations()
            .accept(invitation.clone(), user.id)
            .await?;
        work.sessions().revoke_all(user.id).await?;
        let token = work.sessions().new_session(user.id, method).await?;
        Ok(SessionUpdate::new(
            &token,
            Workspace {
                id: invitation.workspace_id,
                name: invitation.workspace_name,
                slug: invitation.workspace_slug,
                role: invitation.role,
            },
        ))
    }
    .await;
    finish(work, result).await
}
