use crate::{
    context::{Action as ContextAction, Context},
    gtd::Documents,
    project::{ActionStatus, Project, Status as ProjectStatus},
};
use std::{borrow::Cow, collections::HashSet};

pub fn validate(docs: Documents) {
    ValidatorRunner::new()
        .for_all_projects(project_id_is_unique())
        .for_all_projects(project_title_matches_name)
        .for_all_projects(complete_project_has_only_complete_actions)
        .for_all_projects(in_progress_project_has_active_actions)
        .for_all_context_actions(action_link_is_valid)
        .for_all_context_actions(linked_project_is_in_progress)
        .for_all_context_actions(linked_project_contains_action)
        .for_all_context_actions(action_in_project_is_active)
        .for_all_context_actions(linked_action_is_unique())
        .with_ad_hoc(all_active_actions_are_in_a_context)
        .run(&docs);
}

fn project_id_is_unique() -> impl FnMut(&Project) -> Result<(), Cow<'static, str>> {
    let mut project_ids = HashSet::new();

    move |project| {
        if !project_ids.insert(project.id().to_string()) {
            return Err("has a duplicate ID".into());
        }

        Ok(())
    }
}

fn project_title_matches_name(project: &Project) -> Result<(), Cow<'static, str>> {
    let name_title = project.title();

    let body_title = project
        .title
        .try_to_title_string()
        .ok_or("has an invalid title in its body")?;

    if name_title != body_title {
        return Err(format!("has a name \"{}\" that doesn't match its title", body_title).into());
    }

    Ok(())
}

fn complete_project_has_only_complete_actions(project: &Project) -> Result<(), Cow<'static, str>> {
    if project.status != ProjectStatus::Complete {
        return Ok(());
    }

    let are_all_actions_complete = project
        .actions
        .actions()
        .all(|(_, status)| status == ActionStatus::Complete);

    if !are_all_actions_complete {
        return Err("is complete but has at least one uncomplete action".into());
    }

    Ok(())
}

fn in_progress_project_has_active_actions(project: &Project) -> Result<(), Cow<'static, str>> {
    if project.status != ProjectStatus::InProgress {
        return Ok(());
    }

    let has_active_action = project
        .actions
        .actions()
        .filter(|(_, s)| s == &ActionStatus::Active)
        .count()
        >= 1;

    if !has_active_action {
        return Err("is in progress but has no active actions".into());
    }

    Ok(())
}

macro_rules! unwrap_or_ok {
    ($e:expr) => {
        match $e {
            Some(x) => x,
            None => return Ok(()),
        }
    };
}

fn action_link_is_valid(
    action: &ContextAction,
    project: Option<&Project>,
) -> Result<(), Cow<'static, str>> {
    let _action_ref = unwrap_or_ok!(action.to_action_ref());
    if project.is_none() {
        return Err("not a valid link to project".into());
    }

    Ok(())
}

fn linked_project_is_in_progress(
    action: &ContextAction,
    project: Option<&Project>,
) -> Result<(), Cow<'static, str>> {
    let _action_ref = unwrap_or_ok!(action.to_action_ref());
    let project = unwrap_or_ok!(project);

    if project.status != ProjectStatus::InProgress {
        return Err(format!("linked project \"{}\" is not in progress", project.title()).into());
    }

    Ok(())
}

fn linked_project_contains_action(
    action: &ContextAction,
    project: Option<&Project>,
) -> Result<(), Cow<'static, str>> {
    let action_ref = unwrap_or_ok!(action.to_action_ref());
    let project = unwrap_or_ok!(project);

    if project.actions.get_action(&action_ref.action_id).is_none() {
        return Err(format!(
            "linked project \"{}\" doesn't have the action",
            project.title()
        )
        .into());
    }

    Ok(())
}

fn action_in_project_is_active(
    action: &ContextAction,
    project: Option<&Project>,
) -> Result<(), Cow<'static, str>> {
    let action_ref = unwrap_or_ok!(action.to_action_ref());
    let project = unwrap_or_ok!(project);
    let (_, status) = unwrap_or_ok!(project.actions.get_action(&action_ref.action_id));

    if status != ActionStatus::Active {
        return Err(format!(
            "action is not active in linked project \"{}\"",
            project.title()
        )
        .into());
    }

    Ok(())
}

fn linked_action_is_unique(
) -> impl FnMut(&ContextAction, Option<&Project>) -> Result<(), Cow<'static, str>> {
    let mut actions = HashSet::new();
    move |action, _project| {
        if let Some(action_ref) = action.to_action_ref() {
            if !actions.insert(action_ref.clone()) {
                return Err("action is not unique in contexts".into());
            }
        }

        Ok(())
    }
}

fn all_active_actions_are_in_a_context(docs: &Documents) {
    let active_projects = docs
        .projects()
        .filter(|p| p.status == ProjectStatus::InProgress);

    for project in active_projects {
        let active_actions = project.actions.actions().filter_map(|(a, s)| {
            if s == ActionStatus::Active {
                Some(a)
            } else {
                None
            }
        });

        'outer: for action in active_actions {
            let action_id = match &action.id {
                Some(id) => id,
                None => {
                    println!(
                        "Project \"{}\" action is active but isn't in any contexts",
                        project.title()
                    );
                    continue;
                }
            };

            for context in docs.contexts() {
                let action_refs = context.actions().iter().filter_map(|a| a.to_action_ref());

                for action_ref in action_refs {
                    if &action_ref.action_id == action_id {
                        continue 'outer;
                    }
                }
            }

            // TODO: Actually print the action.
            println!(
                "Project \"{}\" action is active but isn't in any contexts",
                project.title()
            );
        }
    }
}

trait ProjectValidator {
    fn validate(&mut self, project: &Project) -> Result<(), Cow<'static, str>>;
}

impl<F> ProjectValidator for F
where
    F: FnMut(&Project) -> Result<(), Cow<'static, str>>,
{
    fn validate(&mut self, project: &Project) -> Result<(), Cow<'static, str>> {
        self(project)
    }
}

trait ContextActionValidator {
    fn validate(
        &mut self,
        action: &ContextAction,
        project: Option<&Project>,
    ) -> Result<(), Cow<'static, str>>;
}

impl<F> ContextActionValidator for F
where
    F: FnMut(&ContextAction, Option<&Project>) -> Result<(), Cow<'static, str>>,
{
    fn validate(
        &mut self,
        action: &ContextAction,
        project: Option<&Project>,
    ) -> Result<(), Cow<'static, str>> {
        self(action, project)
    }
}

trait AdHocValidator {
    fn validate(&mut self, docs: &Documents);
}

impl<F> AdHocValidator for F
where
    F: FnMut(&Documents),
{
    fn validate(&mut self, docs: &Documents) {
        self(docs)
    }
}

pub struct ValidatorRunner<'a> {
    project_validators: Vec<Box<dyn ProjectValidator + 'a>>,
    context_action_validators: Vec<Box<dyn ContextActionValidator + 'a>>,
    ad_hoc_validators: Vec<Box<dyn AdHocValidator + 'a>>,
}

impl<'a> ValidatorRunner<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_all_projects<F>(mut self, validator: F) -> Self
    where
        F: FnMut(&Project) -> Result<(), Cow<'static, str>> + 'a,
    {
        self.project_validators.push(Box::new(validator));
        self
    }

    pub fn for_all_context_actions<F>(mut self, validator: F) -> Self
    where
        F: FnMut(&ContextAction, Option<&Project>) -> Result<(), Cow<'static, str>> + 'a,
    {
        self.context_action_validators.push(Box::new(validator));
        self
    }

    pub fn with_ad_hoc<F>(mut self, validator: F) -> Self
    where
        F: FnMut(&Documents) + 'a,
    {
        self.ad_hoc_validators.push(Box::new(validator));
        self
    }

    pub fn run(mut self, docs: &Documents) {
        for project in docs.projects() {
            self.run_project_validators(project);
        }

        for context in docs.contexts() {
            for action in context.actions() {
                let project = action
                    .to_action_ref()
                    .and_then(|a| docs.project(&a.project_name));
                self.run_context_action_validators(context, action, project);
            }
        }

        self.run_ad_hoc_validators(docs);
    }

    fn run_project_validators(&mut self, project: &Project) {
        let results = self
            .project_validators
            .iter_mut()
            .flat_map(|v| v.validate(project).err())
            .collect::<Vec<_>>();

        if !results.is_empty() {
            println!("{}:", project.name);
            for result in results {
                println!("- {}", result);
            }
        }
    }

    fn run_context_action_validators(
        &mut self,
        context: &Context,
        action: &ContextAction,
        project: Option<&Project>,
    ) {
        let results = self
            .context_action_validators
            .iter_mut()
            .flat_map(|v| v.validate(action, project).err())
            .collect::<Vec<_>>();

        if !results.is_empty() {
            println!("action in {}:", context.name);
            for result in results {
                println!("- {}", result);
            }
        }
    }

    fn run_ad_hoc_validators(&mut self, docs: &Documents) {
        for v in self.ad_hoc_validators.iter_mut() {
            v.validate(docs);
        }
    }
}

impl<'a> Default for ValidatorRunner<'a> {
    fn default() -> Self {
        Self {
            project_validators: Vec::new(),
            context_action_validators: Vec::new(),
            ad_hoc_validators: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        markdown::Fragment,
        project::{ActionId, ActionRef, Name as ProjectName},
    };
    use pulldown_cmark::Event;

    mod project_id_is_unique {
        use super::*;

        #[test]
        fn different_id_same_name_is_unique() {
            let project_a = Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n",
            )
            .unwrap();
            let project_b = Project::parse(
                "197001011200 Project title",
                "# Project title\n#in-progress\n",
            )
            .unwrap();

            let mut validator = project_id_is_unique();
            let _ = validator(&project_a);
            let res = validator(&project_b);
            assert!(res.is_ok());
        }

        #[test]
        fn same_id_different_name_is_not_unique() {
            let project_a = Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n",
            )
            .unwrap();
            let project_b = Project::parse(
                "197001010000 Other project",
                "# Other project\n#in-progress\n",
            )
            .unwrap();

            let mut validator = project_id_is_unique();
            let _ = validator(&project_a);
            let res = validator(&project_b);
            assert!(res.is_err());
        }
    }

    mod project_title_matches_name {
        use super::*;

        #[test]
        fn same_title_matches() {
            let project = Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n",
            )
            .unwrap();

            let res = project_title_matches_name(&project);
            assert!(res.is_ok());
        }

        #[test]
        fn different_title_doesnt_match() {
            let project = Project::parse(
                "197001010000 Project title",
                "# Other project\n#in-progress\n",
            )
            .unwrap();

            let res = project_title_matches_name(&project);
            assert!(res.is_err());
        }
    }

    mod complete_project_has_only_complete_actions {
        use super::*;

        #[test]
        fn noncomplete_project_is_ok() {
            let project = Project::parse(
                "197001010000 Project title",
                "# Other project\n#in-progress\n",
            )
            .unwrap();

            let res = complete_project_has_only_complete_actions(&project);
            assert!(res.is_ok());
        }

        #[test]
        fn complete_project_with_no_actions_is_ok() {
            let project =
                Project::parse("197001010000 Project title", "# Project title\n#complete\n")
                    .unwrap();

            let res = complete_project_has_only_complete_actions(&project);
            assert!(res.is_ok());
        }

        #[test]
        fn complete_project_with_complete_actions_is_ok() {
            let project = Project::parse(
                "197001010000 Project title",
                "# Project title\n#complete\n\n## Actions\n\n### Complete\n\n- Action\n\n",
            )
            .unwrap();

            let res = complete_project_has_only_complete_actions(&project);
            assert!(res.is_ok());
        }

        #[test]
        fn complete_project_with_incomplete_action_is_err() {
            let project = Project::parse(
                "197001010000 Project title",
                "# Project title\n#complete\n\n## Actions\n\n### Active\n\n- Action one\n\n### Complete\n\n- Action two\n\n",
            )
            .unwrap();

            let res = complete_project_has_only_complete_actions(&project);
            assert!(res.is_err());
        }
    }

    mod in_progress_project_has_active_actions {
        use super::*;

        #[test]
        fn non_active_project_is_ok() {
            let project =
                Project::parse("197001010000 Project title", "# Project title\n#complete\n")
                    .unwrap();

            let res = in_progress_project_has_active_actions(&project);
            assert!(res.is_ok());
        }

        #[test]
        fn active_project_with_active_actions_is_ok() {
            let project = Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n\n## Actions\n\n### Active\n\n- Action one\n\n### Complete\n\n- Action two\n\n",
            )
            .unwrap();

            let res = in_progress_project_has_active_actions(&project);
            assert!(res.is_ok());
        }

        #[test]
        fn active_project_with_no_active_actions_is_err() {
            let project = Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n\n## Actions\n\n### Complete\n\n- Action two\n\n",
            )
            .unwrap();

            let res = in_progress_project_has_active_actions(&project);
            assert!(res.is_err());
        }
    }

    mod action_link_is_valid {
        use super::*;

        #[test]
        fn literal_action_is_ok() {
            let action = ContextAction::Literal(Fragment::from_events(vec![Event::Text(
                "Action text".into(),
            )]));

            let res = action_link_is_valid(&action, None);
            assert!(res.is_ok());
        }

        #[test]
        fn extant_project_is_ok() {
            let action = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let project = &Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n",
            )
            .unwrap();

            let res = action_link_is_valid(&action, Some(project));
            assert!(res.is_ok());
        }

        #[test]
        fn nonexistent_project_is_err() {
            let action = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });

            let res = action_link_is_valid(&action, None);
            assert!(res.is_err());
        }
    }

    mod linked_project_is_in_progress {
        use super::*;

        #[test]
        fn literal_action_is_ok() {
            let action = ContextAction::Literal(Fragment::from_events(vec![Event::Text(
                "Action text".into(),
            )]));

            let res = linked_project_is_in_progress(&action, None);
            assert!(res.is_ok());
        }

        #[test]
        fn in_progress_project_is_ok() {
            let action = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let project = &Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n",
            )
            .unwrap();

            let res = linked_project_is_in_progress(&action, Some(project));
            assert!(res.is_ok());
        }

        #[test]
        fn non_in_progress_project_is_err() {
            let action = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let project =
                &Project::parse("197001010000 Project title", "# Project title\n#complete\n")
                    .unwrap();

            let res = linked_project_is_in_progress(&action, Some(project));
            assert!(res.is_err());
        }
    }

    mod linked_project_contains_action {
        use super::*;

        #[test]
        fn literal_action_is_ok() {
            let action = ContextAction::Literal(Fragment::from_events(vec![Event::Text(
                "Action text".into(),
            )]));

            let res = linked_project_contains_action(&action, None);
            assert!(res.is_ok());
        }

        #[test]
        fn project_containing_action_is_ok() {
            let action = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let project = &Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n\n## Actions\n\n### Active\n\n- Action text ^abcdef"
            ).unwrap();

            let res = linked_project_contains_action(&action, Some(project));
            assert!(res.is_ok());
        }

        #[test]
        fn project_not_containing_action_is_err() {
            let action = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let project = &Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n",
            )
            .unwrap();

            let res = linked_project_contains_action(&action, Some(project));
            assert!(res.is_err());
        }

        #[test]
        fn project_containing_same_action_with_different_id_is_err() {
            let action = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let project = &Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n\n## Actions\n\n### Active\n\n- Action text ^ghijkl"
            ).unwrap();

            let res = linked_project_contains_action(&action, Some(project));
            assert!(res.is_err());
        }
    }

    mod action_in_project_is_active {
        use super::*;

        #[test]
        fn literal_action_is_ok() {
            let action = ContextAction::Literal(Fragment::from_events(vec![Event::Text(
                "Action text".into(),
            )]));

            let res = action_in_project_is_active(&action, None);
            assert!(res.is_ok());
        }

        #[test]
        fn active_action_is_ok() {
            let action = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let project = &Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n\n## Actions\n\n### Active\n\n- Action text ^abcdef"
            ).unwrap();

            let res = action_in_project_is_active(&action, Some(project));
            assert!(res.is_ok());
        }

        #[test]
        fn inactive_action_is_err() {
            let action = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let project = &Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n\n## Actions\n\n### Complete\n\n- Action text ^abcdef"
            ).unwrap();

            let res = action_in_project_is_active(&action, Some(project));
            assert!(res.is_err());
        }
    }

    mod linked_action_is_unique {
        use super::*;

        #[test]
        fn unique_action_is_ok() {
            let action_a = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let action_b = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdeg".into()),
            });
            let project = &Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n\n## Actions\n\n### Active\n\n- Action text ^abcdef\n- Other ^abcdeg"
            ).unwrap();

            let mut validator = linked_action_is_unique();
            let _ = validator(&action_a, Some(project));
            let res = validator(&action_b, Some(project));
            assert!(res.is_ok());
        }

        #[test]
        fn repeated_action_is_err() {
            let action_a = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let action_b = ContextAction::Reference(ActionRef {
                project_name: ProjectName::new("197001010000 Project title".into()).unwrap(),
                action_id: ActionId::new("abcdef".into()),
            });
            let project = &Project::parse(
                "197001010000 Project title",
                "# Project title\n#in-progress\n\n## Actions\n\n### Active\n\n- Action text ^abcdef"
            ).unwrap();

            let mut validator = linked_action_is_unique();
            let _ = validator(&action_a, Some(project));
            let res = validator(&action_b, Some(project));
            assert!(res.is_err());
        }
    }
}
