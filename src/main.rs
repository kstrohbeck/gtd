use self::action_list::ActionList;
use self::project::{Project, Status as ProjectStatus};
use argh::FromArgs;
use std::{collections::HashSet, convert::AsRef, env, fs, path::Path};

mod action_list;
mod markdown;
mod parser;
mod project;

/// Task management application.
#[derive(Debug, FromArgs)]
struct Gtd {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Validate(Validate),
}

/// Validates all projects and lists.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "validate")]
struct Validate {}

fn main() {
    let gtd: Gtd = argh::from_env();
    let cur_dir = env::current_dir().unwrap();

    match gtd.subcommand {
        Subcommand::Validate(_v) => {
            fn validate_project<'a>(project: &'a Project, ids: &mut HashSet<&'a str>) {
                fn validate_id<'a>(
                    project: &'a Project,
                    ids: &mut HashSet<&'a str>,
                ) -> Result<(), String> {
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
                    let are_all_actions_complete = project.actions.iter().all(|(x, _)| *x);
                    if project.status == ProjectStatus::Complete && !are_all_actions_complete {
                        Err(format!(
                            "{} is marked complete but has at least one uncompleted action",
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

            let docs = Documents::load(&cur_dir);
            let mut ids = HashSet::new();

            for project in &docs.projects {
                validate_project(project, &mut ids);
            }

            for project in docs
                .projects
                .iter()
                .filter(|p| p.status == ProjectStatus::InProgress)
            {
                let mut has_active_action = false;
                for action in project.actions.iter().map(|(_, f)| f) {
                    for ctx in &docs.action_list.contexts {
                        for act in &ctx.actions {
                            if action == &act.text {
                                has_active_action = true;
                            }
                        }
                    }
                }

                if !has_active_action {
                    println!(
                        "{} is in project list but has no actions in action list",
                        project
                    );
                }
            }

            for context in &docs.action_list.contexts {
                for action in &context.actions {
                    if let Some(link) = &action.project {
                        let link = link as &str;
                        if let Some(project) = docs.projects.iter().find(|p| p.filename == link) {
                            if project.status != ProjectStatus::InProgress {
                                println!("{} is referenced in the action list but is not in the project list", link);
                            }

                            let mut has_action = false;
                            for (done, act) in &project.actions {
                                if &action.text == act {
                                    has_action = true;
                                    if *done {
                                        println!("{} has an action marked as done that is in the action list", link);
                                    }
                                }
                            }

                            if !has_action {
                                println!("{} is referenced in the action list but does not have the referencing action", link);
                            }
                        } else {
                            println!("{} is not a valid link to project in action list", link);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Debug)]
struct Documents {
    action_list: ActionList,
    projects: Vec<Project>,
}

impl Documents {
    fn load<P: AsRef<Path>>(cur_dir: P) -> Self {
        fn load_action_list<P: AsRef<Path>>(cur_dir: P) -> ActionList {
            let path = cur_dir.as_ref().join("Next Actions.md");
            let text = fs::read_to_string(&path).unwrap();
            ActionList::parse(&text).unwrap()
        }

        fn load_projects<P: AsRef<Path>>(cur_dir: P) -> impl Iterator<Item = Project> {
            let project_dir = cur_dir.as_ref().join("Projects");
            fs::read_dir(&project_dir).unwrap().flat_map(|e| {
                let path = e.ok()?.path();
                if path.is_dir() {
                    return None;
                }

                let text = fs::read_to_string(&path).ok()?;
                let name = path.file_stem()?.to_str()?.to_string();
                Project::parse(name, &text).ok()
            })
        }

        let cur_dir = cur_dir.as_ref();
        Self {
            action_list: load_action_list(cur_dir),
            projects: load_projects(cur_dir).collect(),
        }
    }
}
