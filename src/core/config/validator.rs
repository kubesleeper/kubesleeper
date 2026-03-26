use crate::core::config::{Config};
use crate::core::k8s::identifier::Identifier;
use std::collections::HashSet;
use std::fmt::{Display};


#[derive(Debug,Clone)]
pub enum ResourceType {
    Service,
    Deployment,
}

#[derive(Debug)]
pub struct ConflictError{
    from_group_name: String,
    from_id: Identifier,
    to_group_name: String,
    to_id: Identifier,
    resource_type:ResourceType
}
impl Display for ConflictError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Field '{}.{:?}.{}' collide with '{}.{:?}.{}'", 
            self.from_group_name,
            self.resource_type,
            self.from_id,
            self.to_group_name,
            self.resource_type,
            self.to_id
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ValidatorError {
    #[error("")]
    ConflictErrors(Vec<ConflictError>)
}


/// Check if there are no overlapping Identifier between groups
///
/// groups:
///   group1:
///     deployments:
///       - a/b ─────┐
///   group2:        | duplicates, explicite
///     deployments: |  
///       - a/b ─────┘
/// 
/// groups:
///   group1:
///     deployments:
///       - a/* ─────┐
///   group2:        | duplicates, implicite : a/* already cover a/b
///     deployments: |  
///       - a/b ─────┘
/// 
pub fn check_collisions(config: &Config, resource_type: ResourceType) -> Vec<ConflictError> {
    let mut errors : Vec<ConflictError>= Vec::new();
    let mut reported_pairs = HashSet::new();
    
    let all_ids: Vec<(&String, &Identifier)> = match resource_type {
        ResourceType::Service => {
            config.groups
                .iter()
                .flat_map(|(name, group)| group.services.iter().map(move |id| (name, id)))
                .collect()
        },
        ResourceType::Deployment => {
            config.groups
                .iter()
                .flat_map(|(name, group)| group.deployments.iter().map(move |id| (name, id)))
                .collect()
        }
    };
    
    
    
    
    let wildcards: Vec<_> = all_ids.iter().filter(|(_, id)| id.name.is_star()).collect();
    let explicits: Vec<_> = all_ids.iter().filter(|(_, id)| !id.name.is_star()).collect();

    // --- PHASE 1 : Priorité aux Wildcards (*) ---
    // Un "*" peut entrer en collision avec un autre "*" ou un nom explicite
    for (g_w, id_w) in &wildcards {
        for (g_any, id_any) in &all_ids {
            // Ne pas se comparer soi-même
            if std::ptr::eq(*g_w, *g_any) && std::ptr::eq(*id_w, *id_any) { continue; }

            if id_w.namespace == id_any.namespace {
                if !reported_pairs.contains(&(id_w, id_any)) && !reported_pairs.contains(&(id_any, id_w)) {
                    errors.push( ConflictError{
                        from_group_name: format!("{}",g_w),
                        from_id: id_w.to_owned().to_owned(),
                        to_group_name: format!("{}",g_any),
                        to_id: id_any.to_owned().to_owned(),
                        resource_type: resource_type.clone()
                    });
                    reported_pairs.insert((id_w, id_any));
                }
            }
        }
    }

    // --- PHASE 2 : Collisions Explicites (sans les *) ---
    for i in 0..explicits.len() {
        for j in i + 1..explicits.len() {
            let (g1, id1) = explicits[i];
            let (g2, id2) = explicits[j];

            if id1.namespace == id2.namespace && id1.name == id2.name {
                // On vérifie si cet ID n'a pas déjà été "couvert" par un message de Phase 1
                // (Optionnel selon ta préférence, ici on ne rapporte le doublon que s'il est "nu")
                if !reported_pairs.iter().any(|(w, any)| std::ptr::eq(*any, id1) || std::ptr::eq(*any, id2)) {
                    errors.push( ConflictError{
                        from_group_name: format!("{}",g1),
                        from_id: id1.to_owned().to_owned(),
                        to_group_name: format!("{}",g2),
                        to_id: id2.to_owned().to_owned(),
                        resource_type: resource_type.clone()
                    });
                }
            }
        }
    }
    errors
}


pub fn validator(config: &Config) -> Result<(), ValidatorError> {
    
    let mut errors = check_collisions(config, ResourceType::Deployment);
    errors.append(&mut check_collisions(config, ResourceType::Service));
    
    match errors.is_empty() {
        true => Ok(()),
        false => Err(ValidatorError::ConflictErrors(errors))
    }

}

