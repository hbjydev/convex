use convex_core::{Error as AppError, Result as AppResult};
use uuid::Uuid;

pub use convex_core::validation::name;

pub fn workspace_slug(value: &str) -> AppResult<String> {
    let value = value.trim().to_ascii_lowercase();
    if [
        "account",
        "api",
        "assets",
        "auth",
        "invite",
        "setup",
        "system",
        "workspaces",
    ]
    .contains(&value.as_str())
    {
        return Err(AppError::validation(
            "This workspace URL is reserved for the application. Choose another slug.",
        ));
    }
    if !(3..=63).contains(&value.len())
        || value.split('-').any(|part| {
            part.is_empty()
                || !part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
    {
        return Err(AppError::validation(
            "Workspace URLs need 3–63 lowercase letters or numbers, separated by single hyphens.",
        ));
    }
    Ok(value)
}

pub fn team_key(value: &str) -> AppResult<String> {
    let value = value.trim().to_ascii_uppercase();
    if !(2..=10).contains(&value.len())
        || !value.as_bytes()[0].is_ascii_uppercase()
        || !value
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        return Err(AppError::validation(
            "Team keys need 2–10 letters or numbers, starting with a letter.",
        ));
    }
    Ok(value)
}

pub fn manage(role: &str) -> AppResult<()> {
    if role == "owner" || role == "admin" {
        Ok(())
    } else {
        Err(AppError::forbidden())
    }
}

pub fn role(value: &str, owners: bool) -> AppResult<()> {
    if matches!(value, "admin" | "member") || (owners && value == "owner") {
        Ok(())
    } else {
        Err(AppError::validation("Choose an allowed workspace role."))
    }
}

pub fn workspace_id(value: &str) -> AppResult<Uuid> {
    value.parse().map_err(|_| AppError::not_found())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_slugs_are_url_safe() {
        assert_eq!(workspace_slug(" Acme-2026 ").unwrap(), "acme-2026");
        assert!(workspace_slug(&"a".repeat(63)).is_ok());
        for invalid in [
            "",
            "ab",
            "-acme",
            "acme-",
            "acme--team",
            "acme/team",
            "a b",
            "café",
        ] {
            assert!(workspace_slug(invalid).is_err(), "accepted {invalid:?}");
        }
        assert!(workspace_slug(&"a".repeat(64)).is_err());
        for reserved in [
            "account",
            "api",
            "assets",
            "auth",
            "invite",
            "setup",
            "system",
            "workspaces",
        ] {
            assert!(workspace_slug(reserved).is_err());
        }
    }
}
