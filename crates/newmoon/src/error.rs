//! Errors. The C# code threw `UserError` (Game\ClientReader.cs) for messages meant for the person (shown as they are,
//! no stack trace) and let anything else surface as an error. Here every fallible function returns
//! [`Result`] (anyhow); a message for the person is a [`UserError`] inside it, found again with
//! `err.downcast_ref::<UserError>()`.

/// A message for the person using the tool (C# `UserError`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct UserError(pub String);

impl UserError {
    pub fn new(message: impl Into<String>) -> Self {
        UserError(message.into())
    }
}

/// `anyhow::Result`, the result type of the whole library.
pub type Result<T, E = anyhow::Error> = std::result::Result<T, E>;

/// `Err(user_error("..."))?` / `return Err(user_error(format!(...)))`: an `anyhow::Error` holding a [`UserError`].
pub fn user_error(message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(UserError::new(message))
}

/// True when the error is (or wraps) a [`UserError`].
pub fn is_user_error(e: &anyhow::Error) -> bool {
    e.downcast_ref::<UserError>().is_some()
}
