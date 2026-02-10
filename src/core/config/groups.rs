use serde::{Deserialize, Serialize};

use crate::core::resource::identifier::Identifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    name: String,
    deploys: Vec<Identifier>,
    services: Vec<Identifier>,
}
