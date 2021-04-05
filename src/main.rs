use self::gtd::Documents;
use argh::FromArgs;
use std::env;

mod context;
mod gtd;
mod markdown;
mod parser;
mod project;
mod pulldown;
mod validate;

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
        Subcommand::Validate(_opts) => {
            let docs = Documents::load(cur_dir);
            validate::validate(docs.unwrap());
        }
    }
}
