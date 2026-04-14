use crate::core::config::Config;
use crate::core::k8s::identifier::Identifier;

#[derive(Debug, Clone)]
pub enum ResourceType {
    Services,
    Deployments,
}
impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConflictError {
    #[error(
        "'{from_group_name}.{resource_type}.{from_id}' and '{to_group_name}.{resource_type}.{to_id}' target same resource(s)"
    )]
    ExpliciteError {
        from_group_name: String,
        from_id: Identifier,
        to_group_name: String,
        to_id: Identifier,
        resource_type: ResourceType,
    },

    #[error(
        "'{from_group_name}.{resource_type}.{from_id}' already covers '{to_group_name}.{resource_type}.{to_id}' resource"
    )]
    WildcardError {
        from_group_name: String,
        from_id: Identifier,
        to_group_name: String,
        to_id: Identifier,
        resource_type: ResourceType,
    },
}

#[derive(Debug, thiserror::Error)]
pub enum ValidatorError {
    #[error("")]
    ConflictErrors(Vec<ConflictError>),
}
pub fn display_validation_errors(errors: &ValidatorError) -> String {
    match errors {
        ValidatorError::ConflictErrors(conflict_errors) => {
            conflict_errors.iter().fold(String::new(), |acc, val| {
                format!("{}\n  - {}", acc, val.to_string())
            })
        }
    }
}


/// Check if there are no overlapping Identifier between groups
///
/// groups:
///   group1:
///     deployments:
///       - a/b ─────┐
///   group2:        | explicite duplicates
///     deployments: |
///       - a/b ─────┘
///
/// ─────────────────────
///
/// groups:
///   group1:
///     deployments:
///       - a/* ─────┐
///   group2:        | implicite duplicates (a/* already covers a/b)
///     deployments: |
///       - a/b ─────┘
///
pub fn check_collisions(config: &Config, resource_type: ResourceType) -> Vec<ConflictError> {
    let mut errors: Vec<ConflictError> = Vec::new();

    // flatten config, to be in O(n2) instead of O(n4)
    let all_ids: Vec<(&String, &Identifier)> = match resource_type {
        ResourceType::Services => config
            .groups
            .iter()
            .flat_map(|(group)| group.services.iter().map(move |id| (&group.name, id)))
            .collect(),
        ResourceType::Deployments => config
            .groups
            .iter()
            .flat_map(|(group)| group.deployments.iter().map(move |id| (&group.name, id)))
            .collect(),
    };

    for i in 0..all_ids.len() {
        for j in i + 1..all_ids.len() {
            let (g1, id1) = all_ids[i];
            let (g2, id2) = all_ids[j];

            if id1.namespace == id2.namespace && id1.name == id2.name {
                errors.push(ConflictError::ExpliciteError {
                    from_group_name: format!("{}", g1),
                    from_id: id1.to_owned().to_owned(),
                    to_group_name: format!("{}", g2),
                    to_id: id2.to_owned().to_owned(),
                    resource_type: resource_type.clone(),
                });
            }
        }
    }

    let wildcards: Vec<_> = all_ids.iter().filter(|(_, id)| id.name.is_star()).collect();
    let explicits: Vec<_> = all_ids
        .iter()
        .filter(|(_, id)| !id.name.is_star())
        .collect();

    for (g_w, id_w) in &wildcards {
        for (g_e, id_e) in &explicits {
            if id_w.namespace == id_e.namespace {
                errors.push(ConflictError::WildcardError {
                    from_group_name: format!("{}", g_w),
                    from_id: id_w.to_owned().to_owned(),
                    to_group_name: format!("{}", g_e),
                    to_id: id_e.to_owned().to_owned(),
                    resource_type: resource_type.clone(),
                });
            }
        }
    }
    errors
}

pub fn validator(config: &Config) -> Result<(), ValidatorError> {
    let mut errors = check_collisions(config, ResourceType::Deployments);
    errors.append(&mut check_collisions(config, ResourceType::Services));

    match errors.is_empty() {
        true => Ok(()),
        false => Err(ValidatorError::ConflictErrors(errors)),
    }
}
