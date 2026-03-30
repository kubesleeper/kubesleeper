use std::collections::HashMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::core::k8s::identifier::Identifier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub name: String,
    pub deployments: Vec<Identifier>,
    pub services: Vec<Identifier>,
}


#[derive(Deserialize,Serialize)]
struct ConfigPartialGroup {
    pub deployments: Vec<Identifier>,
    pub services: Vec<Identifier>,
}

impl Group {
    
    pub fn deserialize_groups<'de, D>(deserializer: D) -> Result<Vec<Group>, D::Error>
    where
        D: Deserializer<'de>,
    {

        let intermediate_map: HashMap<String, ConfigPartialGroup> = HashMap::deserialize(deserializer)?;
    
        let groups = intermediate_map
            .into_iter()
            .map(|(key, val)| Group {
                name: key,
                deployments: val.deployments,
                services: val.services,
            })
            .collect();
    
        Ok(groups)
    }
    
    
    
    
    pub fn serialize_groups<S>(vec: &[Group], serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {

            let map: HashMap<String, ConfigPartialGroup> = vec
                .iter()
                .map(|g| {
                    (
                        g.name.to_owned(),
                        ConfigPartialGroup {
                            deployments: g.deployments.to_owned(),
                            services: g.services.to_owned(),
                        },
                    )
                })
                .collect();
    
            map.serialize(serializer)
        }
}

