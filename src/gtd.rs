use crate::{context::Context, project::Project};
use std::{convert::AsRef, fs, path::Path};
#[derive(Debug)]
pub struct Documents {
    pub projects: Vec<Project>,
    pub contexts: Vec<Context>,
}

impl Documents {
    pub fn load<P: AsRef<Path>>(cur_dir: P) -> Self {
        fn load_dir<P: AsRef<Path>>(cur_dir: P) -> impl Iterator<Item = (String, String)> {
            fs::read_dir(cur_dir).unwrap().flat_map(|e| {
                let path = e.ok()?.path();
                if path.is_dir() {
                    return None;
                }

                let text = fs::read_to_string(&path).ok()?;
                let name = path.file_stem()?.to_str()?.to_string();
                Some((name, text))
            })
        }

        let cur_dir = cur_dir.as_ref();

        let project_dir = cur_dir.join("Projects");
        let projects = load_dir(project_dir)
            .flat_map(|(name, text)| Project::parse(name, &text).ok())
            .collect();

        let context_dir = cur_dir.join("Contexts");
        let contexts = load_dir(context_dir)
            .flat_map(|(name, text)| Context::parse(name, &text).ok())
            .collect();

        Self { contexts, projects }
    }
}
