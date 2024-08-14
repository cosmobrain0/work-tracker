use std::error::Error;
use tokio::runtime::Runtime;

use super::{LocalProject, Project, ProjectId};

pub trait Config {
    async fn create_new_project(
        &mut self,
        name: String,
        description: String,
    ) -> Result<ProjectId, Box<dyn Error + Send + Sync>>;
    async fn project_ids(&self) -> Result<Vec<ProjectId>, Box<dyn Error + Send + Sync>>;
    async fn project_from_id(
        &self,
        id: ProjectId,
    ) -> Result<Option<Box<dyn Project>>, Box<dyn Error + Send + Sync>>;
    async fn projects(&self) -> Result<Vec<Box<dyn Project>>, Box<dyn Error + Send + Sync>> {
        self.project_ids()
            .filter_map(|x| self.project_from_id(x))
            .collect()
    }
    async fn remove_project(&self, id: ProjectId) -> Result<bool, Box<dyn Error + Send + Sync>>;
}
pub trait ConfigSync: Config {
    fn create_new_project_sync(
        &mut self,
        name: String,
        description: String,
    ) -> Result<ProjectId, Box<dyn Error + Send + Sync>> {
        Runtime::new()
            .unwrap()
            .block_on(self.create_new_project(name, description))
    }
    fn project_ids_sync(&self) -> Result<Vec<ProjectId>, Box<dyn Error + Send + Sync>> {
        Runtime::new().unwrap().block_on(self.project_ids());
    }
    fn project_from_id_sync(
        &self,
        id: ProjectId,
    ) -> Result<Option<Box<dyn Project>>, Box<dyn Error + Send + Sync>> {
        Runtime::new().unwrap().block_on(self.project_from_id(id));
    }
    fn projects_sync(&self) -> Result<Vec<Box<dyn Project>>, Box<dyn Error + Send + Sync>> {
        Runtime::new()
            .unwrap()
            .block_on(self.project_ids())
            .filter_map(|x| self.project_from_id(x))
            .collect()
    }
    fn remove_project_sync(&self, id: ProjectId) -> Result<bool, Box<dyn Error + Send + Sync>> {
        Runtime::new().unwrap().block_on(self.remove_project(id));
    }
}

pub struct LocalConfig {
    last_id: u64,
    projects: Vec<LocalProject>,
}
impl LocalConfig {
    pub fn new() -> Self {
        Self {
            last_id: 0,
            projects: Vec::new(),
        }
    }
}
impl Config for LocalConfig {
    async fn create_new_project(
        &mut self,
        name: String,
        description: String,
    ) -> Result<ProjectId, Box<dyn Error + Send + Sync>> {
        let id = self.last_id + 1;
        self.projects.push(LocalProject::new(name, description, id));
        self.last_id = id;
        Ok(id)
    }

    async fn project_ids(&self) -> Result<Vec<ProjectId>, Box<dyn Error + Send + Sync>> {
        Ok(self.projects.iter().map(|x| x.id()).collect())
    }

    async fn project_from_id(
        &self,
        id: ProjectId,
    ) -> Result<Option<Box<dyn Project>>, Box<dyn Error + Send + Sync>> {
        Ok(self.projects.iter().find(|x| x.id() == id))
    }

    async fn remove_project(&self, id: ProjectId) -> Result<bool, Box<dyn Error + Send + Sync>> {
        Ok(
            match self
                .projects
                .iter()
                .enumerate()
                .find(|(i, x)| x.id() == id)
                .map(|(i, _)| i)
            {
                Some(i) => {
                    self.projects.remove(i);
                    true
                }
                None => false,
            },
        )
    }
}
impl ConfigSync for LocalConfig {}
