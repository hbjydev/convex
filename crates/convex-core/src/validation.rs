use crate::{Error as AppError, Result as AppResult};

pub fn name(value: &str) -> AppResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 100 {
        return Err(AppError::validation(
            "Names must contain between 1 and 100 characters.",
        ));
    }
    Ok(value.into())
}
