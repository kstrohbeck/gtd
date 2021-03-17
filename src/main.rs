use self::project::Project;
use self::project_list::ProjectList;
use self::someday_list::SomedayList;
use argh::FromArgs;
use std::{env, ffi::OsStr, fs};

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
}

/// List orphaned projects.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "orphaned")]
struct Orphaned {}

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
    }
}
