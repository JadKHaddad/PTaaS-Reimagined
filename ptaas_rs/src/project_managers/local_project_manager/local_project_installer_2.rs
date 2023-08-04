use std::path::PathBuf;
use std::{io::Error as IoError, path::Path};
use thiserror::Error as ThisError;
use tokio::fs::{self, File, ReadDir};

use crate::project_managers::process_2::{
    OsProcessArgs, Process, ProcessKillAndWaitError, ProcessRunError, Status, TerminationStatus,
};

pub struct LocalProjectInstaller {
    id: String,
    uploaded_project_dir: PathBuf,
    installed_project_dir: PathBuf,
    project_env_dir: PathBuf,
}

struct FileAndStringPath {
    file: File,
    path: String,
}

struct OsSpecificArgs {
    program: &'static str,
    pip_path: PathBuf,
    first_arg: &'static str,
}

impl LocalProjectInstaller {
    pub fn new(
        id: String,
        uploaded_project_dir: PathBuf,
        installed_project_dir: PathBuf,
        project_env_dir: PathBuf,
    ) -> Self {
        Self {
            id,
            uploaded_project_dir,
            installed_project_dir,
            project_env_dir,
        }
    }

    pub async fn check_and_run_installation(&self) -> Result<(), InstallError> {
        self.check().await.map_err(StartInstallError::CheckFailed)?;

        let uploaded_project_dir = &self.uploaded_project_dir;

        let project_env_dir = &self.project_env_dir;
        let project_env_dir_str =
            project_env_dir
                .to_str()
                .ok_or(StartInstallError::FailedToConvertPathBufToString(
                    self.project_env_dir.clone(),
                ))?;

        let requirements_file_path = self.get_requirements_file_path();
        let requirements_file_path_str = requirements_file_path.to_str().ok_or(
            StartInstallError::FailedToConvertPathBufToString(requirements_file_path.clone()),
        )?;

        let OsSpecificArgs {
            program,
            pip_path,
            first_arg,
        } = self.create_os_specific_args();

        let pip_path_str =
            pip_path
                .to_str()
                .ok_or(StartInstallError::FailedToConvertPathBufToString(
                    pip_path.clone(),
                ))?;

        // Create venv process
        let (venv_process, mut venv_controller) = Process::new(
            String::from("venv_id"),
            String::from("install_venv_process"),
        );
        let venv_process_cmd = format!("python3 -m venv {}", project_env_dir_str);
        let venv_process_args = OsProcessArgs {
            program,
            args: vec![first_arg, &venv_process_cmd],
            current_dir: ".",
            stdout_sender: None,
            stderr_sender: None,
        };

        // Create req process
        let (req_process, mut req_controller) =
            Process::new(String::from("req_id"), String::from("install_req_process"));
        let req_process_cmd = format!("{} install -r {}", pip_path_str, requirements_file_path_str);
        let req_process_args = OsProcessArgs {
            program,
            args: vec![first_arg, &req_process_cmd],
            current_dir: ".",
            stdout_sender: None,
            stderr_sender: None,
        };

        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                let res = venv_controller.cancel().await;

                //let _ = req_controller.cancel().await;
                println!("\n\n\n10 seconds passed!\n\n\n");
                println!("\n\n\nvenv_controller.cancel().await: {:?}\n\n\n", res);
            }

            venv_res = venv_process.run(venv_process_args) => {
                match venv_res {
                    Ok(status) => {
                        match status {
                            Status::Terminated(TerminationStatus::TerminatedSuccessfully) => {
                                tokio::select! {
                                    _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {

                                        let res = req_controller.cancel().await;
                                        println!("\n\n\nreq_controller.cancel().await: {:?}\n\n\n", res);
                                    }

                                    _ = req_process.run(req_process_args) => {

                                    }
                                }
                            },
                            _ => {
                                println!("\n\n\nvenv process failed with status: {:?}\n\n\n", status);
                            }
                        }
                    },
                    Err(e) => {
                        println!("\n\n\nvenv process failed with error: {:?}\n\n\n", e);
                    },
                }
            }


        }

        // TODO: Now we select!
        // 1. wait for stop signal
        // 2. wait for venv process
        // 2.1. wait for pip install process
        // we will need a controller and an arc status for the installation process
        // we will also have to save the state of the installation, so that we know which process to cancel on stop signal
        // might take ownership of self, or use some options and take them and throw errors if already taken
        // this struct is now responsible for io operations, not the Process itself, so we can write io to file and then forward it back to the caller as well (Bubbles :D)

        Ok(())
    }

    async fn delete_environment_dir_if_exists(&self) -> Result<(), IoError> {
        if fs::try_exists(&self.project_env_dir).await? {
            self.delete_environment_dir().await?;
        }

        Ok(())
    }

    async fn delete_environment_dir(&self) -> Result<(), IoError> {
        fs::remove_dir_all(&self.project_env_dir).await
    }

    fn get_requirements_file_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("requirements.txt")
    }

    fn get_locust_dir_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("locust")
    }

    fn get_process_out_file_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("out.txt")
    }

    fn get_process_err_file_path(&self) -> PathBuf {
        self.uploaded_project_dir.join("err.txt")
    }

    /// A 'check' function fails if the project is not valid.
    /// Otherwise it returns Ok(()).
    async fn check(&self) -> Result<(), ProjectCheckError> {
        let uploaded_project_dir = &self.uploaded_project_dir;

        let _ = Self::check_dir_exists_and_not_empty(uploaded_project_dir)
            .await
            .map_err(|err| ProjectCheckError::ProjectDirError(err.into()))?;

        self.check_requirements_txt_exists_and_locust_in_requirements_txt()
            .await?;

        self.check_locust_dir_exists_and_not_empty_and_contains_python_scripts()
            .await
            .map_err(ProjectCheckError::LocustDirError)?;

        Ok(())
    }

    async fn check_dir_exists_and_not_empty(
        dir: &Path,
    ) -> Result<ReadDir, DirExistsAndNotEmptyError> {
        if !fs::try_exists(dir)
            .await
            .map_err(DirExistsAndNotEmptyError::CouldNotCheckIfDirExists)?
        {
            return Err(DirExistsAndNotEmptyError::DirDoesNotExist);
        }

        let mut dir_content = fs::read_dir(dir)
            .await
            .map_err(DirExistsAndNotEmptyError::CouldNotCheckIfDirIsEmpty)?;

        if dir_content
            .next_entry()
            .await
            .map_err(DirExistsAndNotEmptyError::CouldNotCheckIfDirIsEmpty)?
            .is_none()
        {
            return Err(DirExistsAndNotEmptyError::DirIsEmpty);
        }

        Ok(dir_content)
    }

    async fn check_locust_dir_exists_and_not_empty_and_contains_python_scripts(
        &self,
    ) -> Result<(), LocustDirError> {
        let dir = self.get_locust_dir_path();
        let mut dir_content = Self::check_dir_exists_and_not_empty(&dir).await?;

        while let Some(entry) = dir_content
            .next_entry()
            .await
            .map_err(LocustDirError::CouldNotIterateOverLocustDir)?
        {
            if let Some("py") = entry.path().extension().and_then(|ext| ext.to_str()) {
                return Ok(());
            }
        }

        Err(LocustDirError::NoPythonFilesInLocustDir)
    }

    async fn check_requirements_txt_exists_and_locust_in_requirements_txt(
        &self,
    ) -> Result<(), RequirementsError> {
        let requirements_file_path = self.get_requirements_file_path();
        if !fs::try_exists(&requirements_file_path)
            .await
            .map_err(RequirementsError::CouldNotCheckIfRequirementsTxtExists)?
        {
            return Err(RequirementsError::RequirementsTxtDoesNotExist);
        }

        let requirements_file_content = fs::read_to_string(requirements_file_path)
            .await
            .map_err(RequirementsError::CouldNotReadRequirementsTxt)?;

        if !requirements_file_content.contains("locust") {
            return Err(RequirementsError::LocustIsNotInRequirementsTxt);
        }

        Ok(())
    }

    fn create_os_specific_args(&self) -> OsSpecificArgs {
        let (program, pip_path, first_arg) = if cfg!(target_os = "windows") {
            let program = "cmd";
            let pip_path = self.project_env_dir.join("Scripts").join("pip3");
            let first_first_arg = "/C";

            (program, pip_path, first_first_arg)
        } else {
            let program = "bash";
            let pip_path = self.project_env_dir.join("bin").join("pip3");
            let first_first_arg = "-c";

            (program, pip_path, first_first_arg)
        };

        OsSpecificArgs {
            program,
            pip_path,
            first_arg,
        }
    }
}

#[derive(ThisError, Debug)]
pub enum ProjectCheckError {
    #[error("Project dir error: {0}")]
    ProjectDirError(
        #[source]
        #[from]
        ProjectDirError,
    ),
    #[error("Requirements error: {0}")]
    RequirementsError(
        #[source]
        #[from]
        RequirementsError,
    ),
    #[error("Locust dir error: {0}")]
    LocustDirError(
        #[source]
        #[from]
        LocustDirError,
    ),
}

#[derive(ThisError, Debug)]
pub enum ProjectDirError {
    #[error("Could not check if project dir exists: {0}")]
    CouldNotCheckIfProjectDirExists(#[source] IoError),
    #[error("Project dir does not exist")]
    ProjectDirDoesNotExist,
    #[error("Could not check if project dir is empty: {0}")]
    CouldNotCheckIfProjectDirIsEmpty(#[source] IoError),
    #[error("Project dir is empty")]
    ProjectDirIsEmpty,
}

#[derive(ThisError, Debug)]
pub enum RequirementsError {
    #[error("Could not check if requirements.txt exists: {0}")]
    CouldNotCheckIfRequirementsTxtExists(#[source] IoError),
    #[error("requirements.txt does not exist")]
    RequirementsTxtDoesNotExist,
    #[error("Could not read requirements.txt: {0}")]
    CouldNotReadRequirementsTxt(#[source] IoError),
    #[error("Locust is not in requirements.txt")]
    LocustIsNotInRequirementsTxt,
}

#[derive(ThisError, Debug)]
pub enum LocustDirError {
    #[error("Could not check if locust dir exists: {0}")]
    CouldNotCheckIfLocustDirExists(#[source] IoError),
    #[error("Locust dir does not exist")]
    LocustDirDoesNotExist,
    #[error("Could not check if locust dir is empty: {0}")]
    CouldNotCheckIfLocustDirIsEmpty(#[source] IoError),
    #[error("Locust dir is empty")]
    LocustDirIsEmpty,
    #[error("Could not iterate over locust dir: {0}")]
    CouldNotIterateOverLocustDir(#[source] IoError),
    #[error("Locust dir does not contain any python files")]
    NoPythonFilesInLocustDir,
}

#[derive(ThisError, Debug)]
pub enum InstallError {
    #[error("Could not start install: {0}")]
    StartInstallError(
        #[from]
        #[source]
        StartInstallError,
    ),
    #[error("An error occurred: {0}, and could not clean up: {1}")]
    CleanUpError(ErrorThatTriggersCleanUp, #[source] CleanUpError),
}

#[derive(ThisError, Debug)]
pub enum StartInstallError {
    #[error("Could not convert path buf to string: {0}")]
    FailedToConvertPathBufToString(PathBuf),
    #[error("Project is not valid: {0}")]
    CheckFailed(
        #[from]
        #[source]
        ProjectCheckError,
    ),
    #[error("Could not create process: {0}")]
    ProcessCreateError(
        #[from]
        #[source]
        ProcessRunError,
    ),
    #[error("{0}")]
    ErrorThatTriggersCleanUp(
        #[from]
        #[source]
        ErrorThatTriggersCleanUp,
    ),
}

#[derive(ThisError, Debug)]
pub enum ErrorThatTriggersCleanUp {
    #[error("Could not create file: {0}")]
    CreateFileError(
        #[from]
        #[source]
        CreateFileError,
    ),
}

#[derive(ThisError, Debug)]
pub enum CleanUpError {
    #[error("Could not kill process: {0}")]
    CouldNotKillProcess(
        #[source]
        #[from]
        ProcessKillAndWaitError,
    ),
    #[error("Could not delete environment dir: {0}")]
    CouldNotDeleteEnvironment(#[source] IoError),
}

#[derive(ThisError, Debug)]
pub enum DirExistsAndNotEmptyError {
    #[error("Could not check if dir exists: {0}")]
    CouldNotCheckIfDirExists(#[source] IoError),
    #[error("Dir does not exist")]
    DirDoesNotExist,
    #[error("Could not check if dir is empty: {0}")]
    CouldNotCheckIfDirIsEmpty(#[source] IoError),
    #[error("Dir is empty")]
    DirIsEmpty,
}

impl From<DirExistsAndNotEmptyError> for ProjectDirError {
    fn from(dir_exists_and_not_empty_error: DirExistsAndNotEmptyError) -> Self {
        match dir_exists_and_not_empty_error {
            DirExistsAndNotEmptyError::CouldNotCheckIfDirExists(e) => {
                Self::CouldNotCheckIfProjectDirExists(e)
            }
            DirExistsAndNotEmptyError::DirDoesNotExist => Self::ProjectDirDoesNotExist,
            DirExistsAndNotEmptyError::CouldNotCheckIfDirIsEmpty(e) => {
                Self::CouldNotCheckIfProjectDirIsEmpty(e)
            }
            DirExistsAndNotEmptyError::DirIsEmpty => Self::ProjectDirIsEmpty,
        }
    }
}

impl From<DirExistsAndNotEmptyError> for LocustDirError {
    fn from(dir_exists_and_not_empty_error: DirExistsAndNotEmptyError) -> Self {
        match dir_exists_and_not_empty_error {
            DirExistsAndNotEmptyError::CouldNotCheckIfDirExists(e) => {
                Self::CouldNotCheckIfLocustDirExists(e)
            }
            DirExistsAndNotEmptyError::DirDoesNotExist => Self::LocustDirDoesNotExist,
            DirExistsAndNotEmptyError::CouldNotCheckIfDirIsEmpty(e) => {
                Self::CouldNotCheckIfLocustDirIsEmpty(e)
            }
            DirExistsAndNotEmptyError::DirIsEmpty => Self::LocustDirIsEmpty,
        }
    }
}

#[derive(ThisError, Debug)]
pub enum CreateFileError {
    #[error("Could not create file: {0}")]
    CouldNotCreateFile(
        #[source]
        #[from]
        IoError,
    ),
    #[error("Could not convert path buf to string: {0}")]
    FailedToConvertPathBufToString(PathBuf),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use tracing_test::traced_test;

    const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");

    fn get_tests_dir() -> PathBuf {
        Path::new(CRATE_DIR).join("tests_dir")
    }

    fn get_uploaded_projects_dir() -> PathBuf {
        get_tests_dir().join("uploaded_projects")
    }

    fn get_installed_projects_dir() -> PathBuf {
        get_tests_dir().join("installed_projects")
    }

    fn get_environments_dir() -> PathBuf {
        get_tests_dir().join("environments")
    }

    async fn delete_gitkeep(dir: &Path) {
        tokio::fs::remove_file(dir.join(".gitkeep"))
            .await
            .expect("Could not delete .gitkeep");
    }

    async fn restore_gitkeep(dir: &Path) {
        tokio::fs::File::create(dir.join(".gitkeep"))
            .await
            .expect("Could not restore .gitkeep");
    }

    mod install_projects {
        use super::*;

        #[tokio::test]
        #[traced_test]
        // TODO: Not working on CI
        pub async fn install_a_valid_project_and_expect_no_errors() {
            let project_id_and_dir = String::from("valid");
            let uploaded_project_dir = get_uploaded_projects_dir().join(&project_id_and_dir);
            let installed_project_dir = get_installed_projects_dir().join(&project_id_and_dir);
            let project_env_dir = get_environments_dir().join(&project_id_and_dir);

            let installer = LocalProjectInstaller {
                id: project_id_and_dir,
                uploaded_project_dir,
                installed_project_dir,
                project_env_dir,
            };

            match installer.check_and_run_installation().await {
                Ok(_) => {
                    println!("Installation successful");
                }
                Err(e) => panic!("Unexpected error: {}", e),
            }

            installer
                .delete_environment_dir_if_exists()
                .await
                .expect("Could not delete environment dir");

            let error_file_path = installer.get_process_err_file_path();
            let error_output = fs::read_to_string(error_file_path)
                .await
                .expect("Could not read error file");

            let output_file_path = installer.get_process_out_file_path();
            let output_output = fs::read_to_string(output_file_path)
                .await
                .expect("Could not read output file");
        }
    }
}
