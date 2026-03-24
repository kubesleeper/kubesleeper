use crate::core::config::groups::Group;
use crate::core::config::{Config, ConfigError, ValidationError};
use crate::core::resource::identifier::Identifier;
use std::collections::HashMap;

/// Check if there are no overlapping Identifier between groups
///
/// The following example is NOT valid:
///
///     groups:
///       - name: Group1
///         deploys:
///           - a/b
///         ...
///       - name: Group2
///         deploys:
///           - a/b <--- a/b already exist in group 1
///         ...
fn check_duplicate_identifier(valid_groups: &HashMap<String, Group>) -> Vec<ValidationError> {
    let mut resource_conflicts = Vec::<ValidationError>::new();

    let ref_valid_groups: Vec<_> = valid_groups.values().collect();
    for (
        i,
        Group {
            services: r_services,
            deploys: r_deploys,
        },
    ) in ref_valid_groups.iter().enumerate()
    {
        for Group {
            services: l_services,
            deploys: l_deploys,
        } in ref_valid_groups[i + 1..].iter()
        {
            // services
            let dup_services = l_services
                .iter()
                .filter(|e| r_services.contains(e))
                .collect::<Vec<&Identifier>>();
            resource_conflicts.append(
                &mut dup_services
                    .into_iter()
                    .map(|d| ValidationError::ServiceConflict(d.clone()))
                    .collect::<Vec<ValidationError>>(),
            );

            // deploys
            let dup_deploys = l_deploys
                .iter()
                .filter(|e| r_deploys.contains(e))
                .collect::<Vec<&Identifier>>();
            resource_conflicts.append(
                &mut dup_deploys
                    .into_iter()
                    .map(|d| ValidationError::DeployConflict(d.clone()))
                    .collect::<Vec<ValidationError>>(),
            );
        }
    }

    resource_conflicts
}

/// Check if the configuration file is correct
pub fn validator(config: &Config) -> Result<(), ConfigError> {
    let mut validation_errors = Vec::<ValidationError>::new();

    let mut resource_conflicts = check_duplicate_identifier(&config.groups);
    validation_errors.append(&mut resource_conflicts);

    match validation_errors.len() {
        0 => Ok(()),
        _ => Err(ConfigError::ValidationErrors(validation_errors)),
    }
}
