use self::action_list::ActionList;
use self::project::Project;
use self::project_list::ProjectList;
use self::someday_list::SomedayList;
use argh::FromArgs;
use std::{collections::HashSet, convert::AsRef, env, ffi::OsStr, fs, path::Path};

mod action_list;
mod markdown;
mod project;
mod project_list;
mod someday_list;

const COMPLETE_TAG: &str = "complete";

/// Task management application.
#[derive(Debug, FromArgs)]
struct Gtd {
    #[argh(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Orphaned(Orphaned),
    Validate(Validate),
}

/// List orphaned projects.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "orphaned")]
struct Orphaned {}

/// Validates all projects and lists.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "validate")]
struct Validate {}

fn main() {
    let gtd: Gtd = argh::from_env();
    let cur_dir = env::current_dir().unwrap();
    let atom_dir = cur_dir.join("Atoms");

    let project_list_path = cur_dir.join("Projects.md");
    let project_list_text = fs::read_to_string(project_list_path).unwrap();
    let project_list = ProjectList::parse(&project_list_text).unwrap();

    let someday_list_path = cur_dir.join("Someday.md");
    let someday_list_text = fs::read_to_string(someday_list_path).unwrap();
    let someday_list = SomedayList::parse(&someday_list_text).unwrap();

    match gtd.subcommand {
        Subcommand::Orphaned(_o) => {
            let mut found_orphans = false;

            let atoms = fs::read_dir(&atom_dir).unwrap();
            for entry in atoms {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    continue;
                }

                let text = match fs::read_to_string(&path) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                let project = match Project::parse(&text) {
                    Some(p) => p,
                    None => continue,
                };

                let project_filename = path.file_stem().and_then(OsStr::to_str).unwrap();

                if project.tags.iter().any(|t| t == COMPLETE_TAG)
                    || project_list.contains(project_filename)
                    || someday_list.contains(project_filename)
                {
                    continue;
                }

                println!("- {}", project_filename);
                found_orphans = true;
            }
            if !found_orphans {
                println!("No orphaned projects found.");
            }
        }

        Subcommand::Validate(_v) => {
            let action_list = load_action_list(&cur_dir);
            let project_list = load_project_list(&cur_dir);
            let someday_list = load_someday_list(&cur_dir);
            let projects = load_projects(&cur_dir).collect::<Vec<_>>();
            let mut ids = HashSet::new();

            for (filename, project) in &projects {
                let space_idx =
                    filename
                        .char_indices()
                        .find_map(|(i, c)| if c == ' ' { Some(i) } else { None });

                let space_idx = match space_idx {
                    Some(i) => i,
                    None => {
                        println!("{} is not a valid filename", filename);
                        continue;
                    }
                };

                let id = &filename[..space_idx];
                let name = &filename[space_idx + 1..];

                if id.len() != 12 || id.chars().any(|c| !c.is_digit(10)) {
                    println!("{} does not have a valid ID", name);
                }

                let title = match project.title.try_as_title_string() {
                    Some(t) => t,
                    None => {
                        println!("{} has an invalid title header", filename);
                        continue;
                    }
                };

                if name != title {
                    println!("{}'s name and title header don't match", filename);
                    println!("  - name:  {}", name);
                    println!("  - title: {}", title);
                }

                if project.tags.iter().any(|s| s == "complete") {
                    if project_list.contains(filename) {
                        println!("{} is marked complete but is in the project list", filename);
                    }

                    if someday_list.contains(filename) {
                        println!("{} is marked complete but is in the someday list", filename);
                    }
                }

                if project_list.contains(filename) && someday_list.contains(filename) {
                    println!(
                        "{} is in both the project list and the someday list",
                        filename
                    );
                }

                if ids.contains(id) {
                    println!("{} has a duplicate ID", filename);
                } else {
                    ids.insert(id);
                }
            }

            for context in &action_list.contexts {
                for action in &context.actions {}
            }

            // Check if any projects are orphaned.
            // Check if every project in the project list has at least one action in the action list.
            // Check if any action in the action list has a project that isn't in the project list.
            // Check if any link (in the action/projects/someday lists) doesn't point to a valid project.
            // Check if projects list contains more than one reference to the same project.
            // Check if someday list contains more than one reference to the same project.
            // Check if action list contains more than one copy of the same action.
            // If an action in the action list links to a project, make sure that project has that action and it is unchecked.
            // Check if any completed project has actions that are unchecked.
        }
    }
}

fn load_action_list<P: AsRef<Path>>(cur_dir: P) -> ActionList {
    let path = cur_dir.as_ref().join("Action Items.md");
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

fn load_projects<P: AsRef<Path>>(cur_dir: P) -> impl Iterator<Item = (String, Project)> {
    let atom_dir = cur_dir.as_ref().join("Atoms");
    fs::read_dir(&atom_dir).unwrap().flat_map(|e| {
        let path = e.ok()?.path();
        if path.is_dir() {
            return None;
        }

        let text = fs::read_to_string(&path).ok()?;
        let project = Project::parse(&text)?;
        let proj_name = path.file_stem()?.to_str()?.to_string();

        Some((proj_name, project))
    })
}
