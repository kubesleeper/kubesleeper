use serde::{Deserialize, Serialize};

use crate::core::resource::identifier::Identifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub deploys: Vec<Identifier>,
    pub services: Vec<Identifier>,
}

/*
groups:
  1:
   deploys:
    - a/b
    - d/f
   service:
    - a/b
  2:
     deploys:
      - a/b
     service:
      - a/b
*/
