use crate::core::config::ValidationError::{AutoManagedConflict, GroupNameConflict};
use crate::core::config::groups::Group;
use crate::core::config::{Config, ConfigError, ValidationError};
use crate::core::resource::identifier::Identifier;
use crate::core::resource::resource_name::ResourceName;
use std::collections::HashSet;

/// Check if groups does not have the same name
///
/// The following example is NOT valid:
///
///     groups:
///       - name: 1
///         ...
///       - name: 1 <--- Name of this group is already used previously
///         ...
fn check_group_name_conflict(config: &Config) -> (Vec<Group>, Vec<ValidationError>) {
    let mut valid_groups: Vec<Group> = Vec::new();

    let mut group_name_conflicts = Vec::<ValidationError>::new();

    for (i, group) in config.groups.iter().enumerate() {
        if config.groups[i + 1..].iter().any(|g| g.name == group.name) {
            group_name_conflicts.push(GroupNameConflict(group.name.to_string()));
        } else {
            valid_groups.push(group.clone())
        }
    }
    (valid_groups, group_name_conflicts)
}

/// Check if there are no overlapping namespace between group's resources and auto managed namespaces
///
/// The following example is NOT valid:
///
///     auto_managed:
///       - a
///
///     groups:
///       - name: Group1
///         deploys:
///           - a/b <--- namespace already auto managed
///         ...
fn check_auto_managed_names(
    auto_managed_namespace: &HashSet<ResourceName>,
    groups: Vec<Group>,
) -> Vec<ValidationError> {
    let mut auto_managed_conflicts = Vec::<ValidationError>::new();

    for group in groups {
        for id in group.services {
            if auto_managed_namespace.contains(&id.namespace) {
                auto_managed_conflicts.push(AutoManagedConflict {
                    group_name: group.name.to_string(),
                    r#type: "service".to_string(),
                    id: id.clone(),
                    namespace: id.namespace.clone(),
                })
            }
        }

        for id in group.deploys {
            if auto_managed_namespace.contains(&id.namespace) {
                auto_managed_conflicts.push(AutoManagedConflict {
                    group_name: group.name.to_string(),
                    r#type: "deploy".to_string(),
                    id: id.clone(),
                    namespace: id.namespace.clone(),
                })
            }
        }
    }

    auto_managed_conflicts
}

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
fn check_duplicate_identifier(valid_groups: &Vec<Group>) -> Vec<ValidationError> {
    let mut resource_conflicts = Vec::<ValidationError>::new();

    for (
        i,
        Group {
            name: _,
            services: r_services,
            deploys: r_deploys,
        },
    ) in valid_groups.iter().enumerate()
    {
        for Group {
            name: _,
            services: l_services,
            deploys: l_deploys,
        } in valid_groups[i + 1..].iter()
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

    let (valid_groups, mut group_name_conflicts) = check_group_name_conflict(config);
    validation_errors.append(&mut group_name_conflicts);

    let auto_managed_namespace: HashSet<_> =
        HashSet::from_iter(config.auto_managed_namespace.iter().cloned());

    let mut resource_conflicts = check_duplicate_identifier(&valid_groups);
    validation_errors.append(&mut resource_conflicts);

    let mut auto_managed_conflicts =
        check_auto_managed_names(&auto_managed_namespace, valid_groups);
    validation_errors.append(&mut auto_managed_conflicts);

    match validation_errors.len() {
        0 => Ok(()),
        _ => Err(ConfigError::ValidationErrors(validation_errors)),
    }
}
