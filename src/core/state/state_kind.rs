use core::fmt;

use super::StateError;

#[derive(Eq, PartialEq, Debug)]
pub enum StateKind {
    Asleep,
    Awake,
}

impl fmt::Display for StateKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StateKind::Asleep => write!(f, "{}", format!("{:?}",StateKind::Asleep).to_lowercase()),
            StateKind::Awake => write!(f, "{}", format!("{:?}",StateKind::Awake).to_lowercase()),
        }
    }
}

impl TryFrom<&str> for StateKind {
    type Error = StateError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if value == StateKind::Asleep.to_string() {
            return Ok(StateKind::Asleep);
        }
        if value == StateKind::Awake.to_string() {
            return Ok(StateKind::Awake);
        }
        Err(StateError::InvalidStateKindError(format!(
            "Can't parse str '{}' to StateKind, valid str are '{}' and '{}'",
            value,
            StateKind::Asleep.to_string(),
            StateKind::Awake.to_string()
        )))
    }
}
