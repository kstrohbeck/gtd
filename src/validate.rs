use crate::{
    context::{Action as ContextAction, Context},
    gtd::Documents,
    project::{ActionStatus, Project, Status as ProjectStatus},
};
use std::collections::HashSet;

pub fn validate(docs: Documents) {
    ValidatorRunner::new()
        .for_all_projects(project_id_is_unique())
        .for_all_projects(project_title_matches_name)
        .for_all_projects(complete_project_has_only_complete_actions)
        .for_all_projects(in_progress_project_has_active_actions)
        .for_all_context_actions(action_link_is_valid(&docs))
        .for_all_context_actions(linked_project_is_in_progress(&docs))
        .for_all_context_actions(linked_project_contains_action(&docs))
        .for_all_context_actions(action_in_project_is_active(&docs))
        .run(&docs);
}

fn project_id_is_unique() -> impl FnMut(&Project) -> Result<(), &'static str> {
    let mut project_ids = HashSet::new();

    move |project| {
        if !project_ids.insert(project.id().to_string()) {
            return Err("has a duplicate ID");
        }

        Ok(())
    }
}

fn project_title_matches_name(project: &Project) -> Result<(), &'static str> {
    let name_title = project.title();

    let body_title = project
        .title
        .try_to_title_string()
        .ok_or("has an invalid title in its body")?;

    if name_title != body_title {
        return Err("name and body title don't match");
    }

    Ok(())
}

fn complete_project_has_only_complete_actions(project: &Project) -> Result<(), &'static str> {
    if project.status != ProjectStatus::Complete {
        return Ok(());
    }

    let are_all_actions_complete = project
        .actions
        .actions()
        .all(|(_, status)| status == ActionStatus::Complete);

    if !are_all_actions_complete {
        return Err("is complete but has at least one uncomplete action");
    }

    Ok(())
}

fn in_progress_project_has_active_actions(project: &Project) -> Result<(), &'static str> {
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
        return Err("is in progress but has no active actions");
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
    docs: &Documents,
) -> impl FnMut(&Context, &ContextAction) -> Result<(), &'static str> + '_ {
    move |_context, action| {
        let action_ref = unwrap_or_ok!(action.to_action_ref());

        if docs.project(&action_ref.project_name).is_none() {
            return Err("not a valid link to project");
        }

        Ok(())
    }
}

fn linked_project_is_in_progress(
    docs: &Documents,
) -> impl FnMut(&Context, &ContextAction) -> Result<(), &'static str> + '_ {
    move |_context, action| {
        let action_ref = unwrap_or_ok!(action.to_action_ref());
        let project = unwrap_or_ok!(docs.project(&action_ref.project_name));

        if project.status != ProjectStatus::InProgress {
            return Err("linked project is not in progress");
        }

        Ok(())
    }
}

fn linked_project_contains_action(
    docs: &Documents,
) -> impl FnMut(&Context, &ContextAction) -> Result<(), &'static str> + '_ {
    move |_context, action| {
        let action_ref = unwrap_or_ok!(action.to_action_ref());
        let project = unwrap_or_ok!(docs.project(&action_ref.project_name));

        if project.actions.get_action(&action_ref.action_id).is_none() {
            return Err("linked project doesn't have the action");
        }

        Ok(())
    }
}

fn action_in_project_is_active(
    docs: &Documents,
) -> impl FnMut(&Context, &ContextAction) -> Result<(), &'static str> + '_ {
    move |_context, action| {
        let action_ref = unwrap_or_ok!(action.to_action_ref());
        let project = unwrap_or_ok!(docs.project(&action_ref.project_name));
        let (_, status) = unwrap_or_ok!(project.actions.get_action(&action_ref.action_id));

        if status != ActionStatus::Active {
            return Err("action is not active in linked project");
        }

        Ok(())
    }
}

trait ProjectValidator {
    fn validate(&mut self, project: &Project) -> Result<(), &'static str>;
}

impl<F> ProjectValidator for F
where
    F: FnMut(&Project) -> Result<(), &'static str>,
{
    fn validate(&mut self, project: &Project) -> Result<(), &'static str> {
        self(project)
    }
}

trait ContextActionValidator {
    fn validate(&mut self, context: &Context, action: &ContextAction) -> Result<(), &'static str>;
}

impl<F> ContextActionValidator for F
where
    F: FnMut(&Context, &ContextAction) -> Result<(), &'static str>,
{
    fn validate(&mut self, context: &Context, action: &ContextAction) -> Result<(), &'static str> {
        self(context, action)
    }
}

pub struct ValidatorRunner<'a> {
    project_validators: Vec<Box<dyn ProjectValidator + 'a>>,
    context_action_validators: Vec<Box<dyn ContextActionValidator + 'a>>,
}

impl<'a> ValidatorRunner<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn for_all_projects<F>(mut self, validator: F) -> Self
    where
        F: FnMut(&Project) -> Result<(), &'static str> + 'a,
    {
        self.project_validators.push(Box::new(validator));
        self
    }

    pub fn for_all_context_actions<F>(mut self, validator: F) -> Self
    where
        F: FnMut(&Context, &ContextAction) -> Result<(), &'static str> + 'a,
    {
        self.context_action_validators.push(Box::new(validator));
        self
    }

    pub fn run(mut self, docs: &Documents) {
        for project in docs.projects() {
            self.run_project_validators(project);
        }

        for context in docs.contexts() {
            for action in context.actions() {
                self.run_context_action_validators(context, action);
            }
        }
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

    fn run_context_action_validators(&mut self, context: &Context, action: &ContextAction) {
        let results = self
            .context_action_validators
            .iter_mut()
            .flat_map(|v| v.validate(context, action).err())
            .collect::<Vec<_>>();

        if !results.is_empty() {
            println!("action in {}:", context.name);
            for result in results {
                println!("- {}", result);
            }
        }
    }
}

impl<'a> Default for ValidatorRunner<'a> {
    fn default() -> Self {
        Self {
            project_validators: Vec::new(),
            context_action_validators: Vec::new(),
        }
    }
}
