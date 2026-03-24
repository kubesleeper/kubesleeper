use crate::core::config::groups::Group;
use crate::core::config::{Config, ConfigError, ValidationError};
use crate::core::resource::identifier::Identifier;
use std::collections::{HashMap, HashSet};

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
            let mut dup_services = HashSet::new();
            for r_service in r_services {
                for l_service in l_services {
                    if r_service == l_service || r_service.includes(l_service)
                    {
                        dup_services.insert((r_service, l_service));
                    }
                    else if l_service.includes(r_service)
                    {
                        dup_services.insert((l_service, r_service));
                    }
                }
            }
            resource_conflicts.append(
                &mut dup_services
                    .into_iter()
                    .map(|(e, f)| ValidationError::ServiceConflict(e.clone(),f.clone()))
                    .collect::<Vec<ValidationError>>(),
            );

            // deploys
            let mut dup_deploys = HashSet::new();
            for r_deploy in r_deploys {
                for l_deploy in l_deploys {
                    if r_deploy == l_deploy || r_deploy.includes(l_deploy)
                    {
                        dup_deploys.insert((r_deploy, l_deploy));
                    }
                    else if l_deploy.includes(r_deploy)
                    {
                        dup_deploys.insert((r_deploy, r_deploy));
                    }
                }
            }
            resource_conflicts.append(
                &mut dup_deploys
                    .into_iter()
                    .map(|(e, f)| ValidationError::DeployConflict(e.clone(),f.clone()))
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
