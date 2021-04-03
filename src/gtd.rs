use crate::{
    context::{Context, ParseError as ContextParseError},
    project::{ParseError as ProjectParseError, Project},
};
use std::{
    convert::AsRef,
    error::Error,
    fmt, fs,
    io::Error as IoError,
    path::{Path, PathBuf},
};
#[derive(Debug)]
pub struct Documents {
    pub projects: Vec<Project>,
    pub contexts: Vec<Context>,
}

impl Documents {
    pub fn load<P: AsRef<Path>>(cur_dir: P) -> Self {
        let cur_dir = cur_dir.as_ref();
        let loader = Loader::new(cur_dir.to_owned());
        let projects = loader
            .all_project_names()
            .unwrap()
            .map(|name| loader.load_project(&name))
            .collect::<Result<_, _>>()
            .unwrap();
        let contexts = loader
            .all_context_names()
            .unwrap()
            .map(|name| loader.load_context(&name))
            .collect::<Result<_, _>>()
            .unwrap();
        Self { contexts, projects }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Loader {
    root_dir: PathBuf,
    project_dir: PathBuf,
    context_dir: PathBuf,
}

// TODO: Document.
impl Loader {
    pub fn new(root_dir: PathBuf) -> Self {
        let project_dir = root_dir.join("Projects");
        let context_dir = root_dir.join("Contexts");
        Self {
            root_dir,
            project_dir,
            context_dir,
        }
    }

    pub fn all_project_names(&self) -> Result<impl Iterator<Item = ProjectName>, IoError> {
        Self::read_dir(&self.project_dir).map(|i| i.map(ProjectName))
    }

    pub fn all_context_names(&self) -> Result<impl Iterator<Item = ContextName>, IoError> {
        Self::read_dir(&self.context_dir).map(|i| i.map(ContextName))
    }

    fn read_dir(dir: &Path) -> Result<impl Iterator<Item = String>, IoError> {
        let iter = fs::read_dir(dir)?.flat_map(|e| {
            let path = e.ok()?.path();
            if path.is_dir() {
                return None;
            }

            let name = path.file_stem()?.to_str()?.to_string();
            Some(name)
        });
        Ok(iter)
    }

    pub fn load_project(&self, name: &ProjectName) -> Result<Project, LoadProjectError> {
        let name = name.to_inner();
        let text = Self::load_markdown_file(&self.project_dir, &name)?;
        let project = Project::parse(name, &text)?;
        Ok(project)
    }

    pub fn load_context(&self, name: &ContextName) -> Result<Context, LoadContextError> {
        let name = name.to_inner();
        let text = Self::load_markdown_file(&self.context_dir, &name)?;
        let context = Context::parse(name, &text)?;
        Ok(context)
    }

    fn load_markdown_file(dir: &Path, name: &str) -> Result<String, std::io::Error> {
        let mut path = dir.join(name);
        path.set_extension(".md");
        fs::read_to_string(path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProjectName(String);

impl ProjectName {
    fn to_inner(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextName(String);

impl ContextName {
    fn to_inner(&self) -> String {
        self.0.clone()
    }
}

#[derive(Debug)]
pub enum LoadProjectError {
    IoError(IoError),
    ProjectParseError(ProjectParseError<'static>),
}

impl fmt::Display for LoadProjectError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "{}", e),
            Self::ProjectParseError(e) => write!(f, "{}", e),
        }
    }
}

impl Error for LoadProjectError {}

impl From<IoError> for LoadProjectError {
    fn from(error: IoError) -> Self {
        Self::IoError(error)
    }
}

impl<'a> From<ProjectParseError<'a>> for LoadProjectError {
    fn from(error: ProjectParseError<'a>) -> Self {
        Self::ProjectParseError(error.into_static())
    }
}

#[derive(Debug)]
pub enum LoadContextError {
    IoError(IoError),
    ContextParseError(ContextParseError<'static>),
}

impl fmt::Display for LoadContextError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "{}", e),
            Self::ContextParseError(e) => write!(f, "{}", e),
        }
    }
}

impl Error for LoadContextError {}

impl From<IoError> for LoadContextError {
    fn from(error: IoError) -> Self {
        Self::IoError(error)
    }
}

impl<'a> From<ContextParseError<'a>> for LoadContextError {
    fn from(error: ContextParseError<'a>) -> Self {
        Self::ContextParseError(error.into_static())
    }
}
