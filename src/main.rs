use self::action_list::ActionList;
use self::project::Project;
use self::project_list::ProjectList;
use self::someday_list::SomedayList;
use argh::FromArgs;
use std::{collections::HashSet, convert::AsRef, env, fs, path::Path};

mod action_list;
mod markdown;
mod parser;
mod project;
mod project_list;
mod someday_list;

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
            fn validate_project<'a>(
                project: &'a Project,
                docs: &'a Documents,
                ids: &mut HashSet<&'a str>,
            ) {
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

                fn verify_project_is_in_exactly_one_state(
                    project: &Project,
                    project_list: &ProjectList,
                    someday_list: &SomedayList,
                ) -> Result<(), String> {
                    let is_in_project_list = project_list.contains(&project.filename);
                    let is_in_someday_list = someday_list.contains(&project.filename);

                    match (
                        is_in_project_list,
                        is_in_someday_list,
                        project.is_complete(),
                    ) {
                        (false, false, false) => Err(format!(
                            "{} is not marked complete and is not in the project or someday lists",
                            project
                        )),
                        (false, true, true) => Err(format!(
                            "{} is marked complete but is in the someday list",
                            project
                        )),
                        (true, false, true) => Err(format!(
                            "{} is marked complete but is in the project list",
                            project
                        )),
                        (true, true, false) => Err(format!(
                            "{} is in both the project and someday lists",
                            project
                        )),
                        (true, true, true) => Err(format!(
                            "{} is marked complete but is in both the project and someday lists",
                            project
                        )),
                        _ => Ok(()),
                    }
                }

                fn verify_all_actions_complete(project: &Project) -> Result<(), String> {
                    let are_all_actions_complete = project.actions.iter().all(|(x, _)| *x);
                    if project.is_complete() && !are_all_actions_complete {
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
                    verify_project_is_in_exactly_one_state(
                        project,
                        &docs.project_list,
                        &docs.someday_list,
                    ),
                    verify_all_actions_complete(project),
                ];

                for err in validations.into_iter().filter_map(|x| x.err()) {
                    println!("{}", err);
                }
            }

            let docs = Documents::load(&cur_dir);
            let mut project_links = HashSet::new();
            let mut ids = HashSet::new();

            for project in &docs.projects {
                project_links.insert(project.filename.as_str());
                validate_project(project, &docs, &mut ids);
            }

            let mut project_list_links = HashSet::new();

            for link in &docs.project_list.items {
                let link = link as &str;

                if let Some(project) = docs.projects.iter().find(|p| p.filename == link) {
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
                } else {
                    println!("{} is an invalid link in the project list", link);
                }

                if !project_list_links.insert(link) {
                    println!("{} is duplicated in the project list", link);
                }
            }

            let mut someday_list_links = HashSet::new();

            for link in docs.someday_list.items.iter().filter_map(|i| i.link()) {
                if !project_links.contains(link) {
                    println!("{} is an invalid link in the someday list", link);
                }

                if !someday_list_links.insert(link) {
                    println!("{} is duplicated in the someday list", link);
                }
            }

            for context in &docs.action_list.contexts {
                for action in &context.actions {
                    if let Some(link) = &action.project {
                        let link = link as &str;
                        if let Some(project) = docs.projects.iter().find(|p| p.filename == link) {
                            if !docs.project_list.contains(link) {
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
    project_list: ProjectList,
    someday_list: SomedayList,
    projects: Vec<Project>,
}

impl Documents {
    fn load<P: AsRef<Path>>(cur_dir: P) -> Self {
        fn load_action_list<P: AsRef<Path>>(cur_dir: P) -> ActionList {
            let path = cur_dir.as_ref().join("Next Actions.md");
            let text = fs::read_to_string(&path).unwrap();
            ActionList::parse(&text).unwrap()
        }

        fn load_project_list<P: AsRef<Path>>(cur_dir: P) -> ProjectList {
            let path = cur_dir.as_ref().join("Projects.md");
            let text = fs::read_to_string(&path).unwrap();
            ProjectList::parse(&text).unwrap()
        }

        fn load_someday_list<P: AsRef<Path>>(cur_dir: P) -> SomedayList {
            let path = cur_dir.as_ref().join("Someday.md");
            let text = fs::read_to_string(&path).unwrap();
            SomedayList::parse(&text).unwrap()
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
            project_list: load_project_list(cur_dir),
            someday_list: load_someday_list(cur_dir),
            projects: load_projects(cur_dir).collect(),
        }
    }
}
