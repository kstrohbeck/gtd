use self::project::Project;
use argh::FromArgs;
use std::fs::read_to_string;

mod action_list;
mod markdown;
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
    Orphaned(Orphaned),
}

/// Find orphaned projects.
#[derive(Debug, FromArgs)]
#[argh(subcommand, name = "orphaned")]
struct Orphaned {}

fn main() {
    let gtd: Gtd = argh::from_env();
    match gtd.subcommand {
        Subcommand::Orphaned(o) => {
            println!("Looking for orphaned projects.");
            // TODO: Need to be able to walk the directory to find projects.
        }
    }
}
