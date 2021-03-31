use crate::{
    context::Action as ContextAction,
    gtd::Documents,
    project::{ActionStatus, Project, Status as ProjectStatus},
};
use std::collections::HashSet;

pub fn validate(docs: Documents) {
    fn validate_project<'a>(project: &'a Project, ids: &mut HashSet<&'a str>) {
        fn validate_id<'a>(project: &'a Project, ids: &mut HashSet<&'a str>) -> Result<(), String> {
            let id = project
                .id()
                .ok_or_else(|| format!("{} has an invalid ID", project))?;

            if !ids.insert(id) {
                return Err(format!("{} has a duplicate ID", project));
            }

            Ok(())
        }

        fn validate_title(project: &Project) -> Result<(), String> {
            let name_title = project
                .name()
                .ok_or_else(|| format!("{} has an invalid title in its name", project))?;

            let body_title = project
                .title
                .try_as_title_string()
                .ok_or_else(|| format!("{} has an invalid title in its body", project))?;

            if name_title != body_title {
                return Err(format!("{}'s name and body title don't match", project));
            }

            Ok(())
        }

        fn verify_all_actions_complete(project: &Project) -> Result<(), String> {
            let are_all_actions_complete = project
                .actions
                .actions()
                .all(|(_, status)| status == ActionStatus::Complete);
            if project.status == ProjectStatus::Complete && !are_all_actions_complete {
                Err(format!(
                    "{} is complete but has at least one uncompleted action",
                    project
                ))
            } else {
                Ok(())
            }
        }

        let validations = vec![
            validate_id(project, ids),
            validate_title(project),
            verify_all_actions_complete(project),
        ];

        for err in validations.into_iter().filter_map(|x| x.err()) {
            println!("{}", err);
        }
    }

    let mut ids = HashSet::new();

    for project in &docs.projects {
        validate_project(project, &mut ids);
    }

    for project in docs
        .projects
        .iter()
        .filter(|p| p.status == ProjectStatus::InProgress)
    {
        let has_active_action = project
            .actions
            .actions()
            .filter(|(_, s)| s == &ActionStatus::Active)
            .count()
            >= 1;

        if !has_active_action {
            println!(
                "{} is in progress but has no actions in any context",
                project
            );
        }
    }

    for context in &docs.contexts {
        let ctx_title = context.title.try_as_title_string().unwrap();
        let linked_actions = context.actions.iter().filter_map(|a| match a {
            ContextAction::Reference(block_ref) => Some(block_ref),
            ContextAction::Literal(_) => None,
        });
        for action in linked_actions {
            if let Some(project) = docs.projects.iter().find(|p| p.filename == action.link) {
                if project.status != ProjectStatus::InProgress {
                    println!(
                        "{} has a next action in {} but is not in progress",
                        action.link, ctx_title
                    );
                }

                if let Some((_act, act_status)) = project.actions.get_action(&action.id) {
                    if act_status != ActionStatus::Active {
                        println!(
                            "{} has a next action in {} that isn't in Active",
                            action.link, ctx_title
                        );
                    }
                } else {
                    println!(
                        "{} is referenced in {} but does not have the referencing action",
                        action.link, ctx_title
                    );
                }
            } else {
                println!(
                    "{} is not a valid link to project in {}",
                    action.link, ctx_title
                );
            }
        }
    }
}
