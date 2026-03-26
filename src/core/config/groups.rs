use serde::{Deserialize, Serialize};

use crate::core::k8s::identifier::Identifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub deploys: Vec<Identifier>,
    pub services: Vec<Identifier>,
}
