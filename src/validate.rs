use crate::{
    context::Action as ContextAction,
    gtd::Documents,
    project::{ActionStatus, Project, Status as ProjectStatus},
};
use std::collections::HashSet;

pub fn validate(docs: Documents) {
    fn validate_project<'a>(project: &'a Project, project_ids: &HashSet<&'a str>) {
        fn validate_id<'a>(
            project: &'a Project,
            project_ids: &HashSet<&'a str>,
        ) -> Result<(), String> {
            if !project_ids.contains(project.id()) {
                return Err(format!("{} has a duplicate ID", project));
            }

            Ok(())
        }

        fn validate_title(project: &Project) -> Result<(), String> {
            let name_title = project.title();

            let body_title = project
                .title
                .try_to_title_string()
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
            validate_id(project, project_ids),
            validate_title(project),
            verify_all_actions_complete(project),
        ];

        for err in validations.into_iter().filter_map(|x| x.err()) {
            println!("{}", err);
        }
    }

    let project_ids = docs.projects().map(|p| p.id()).collect();

    for project in docs.projects() {
        validate_project(project, &project_ids);
    }

    for project in docs
        .projects()
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

    for context in docs.contexts() {
        let ctx_title = context.title.try_to_title_string().unwrap();
        let linked_actions = context
            .actions()
            .iter()
            .filter_map(ContextAction::to_action_ref);

        for action in linked_actions {
            let project = match docs.project(&action.project_name) {
                Some(p) => p,
                None => {
                    println!(
                        "{} is not a valid link to project in {}",
                        action.project_name, ctx_title
                    );
                    continue;
                }
            };

            if project.status != ProjectStatus::InProgress {
                println!(
                    "{} has a next action in {} but is not in progress",
                    action.project_name, ctx_title
                );
            }

            if let Some((_act, act_status)) = project.actions.get_action(&action.action_id) {
                if act_status != ActionStatus::Active {
                    println!(
                        "{} has a next action in {} that isn't in Active",
                        action.project_name, ctx_title
                    );
                }
            } else {
                println!(
                    "{} is referenced in {} but does not have the referencing action",
                    action.project_name, ctx_title
                );
            }
        }
    }
}
