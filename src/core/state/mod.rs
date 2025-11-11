use crate::core::controller::error::ControllerError;

pub mod notification;
pub mod state;
pub mod state_kind;

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("LockError : {0}")]
    LockError(String),

    #[error(transparent)]
    ControllerError(#[from] ControllerError),

    #[error("Invalid State Kind: {0}")]
    InvalidStateKindError(String),
}
