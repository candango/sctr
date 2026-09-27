use std::{path::PathBuf, time::Duration};

use thiserror::Error;

use crate::{
    registry::{Action, Registry, RegistryError},
    systemd::{SystemdClient, SystemdError},
};

const DEFAULT_REGISTRY: &str = "/etc/sctr/registry.json";
const SYSTEMD_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Error)]
pub(crate) enum RuntimeError {
    #[error("cannot determine the current actor UID: {0}")]
    Actor(String),
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error(transparent)]
    Systemd(#[from] SystemdError),
}

#[derive(Debug)]
pub(crate) struct Runtime {
    registry_path: PathBuf,
    actor_uid: u32,
    systemd: SystemdClient,
}

impl Runtime {
    pub(crate) fn system() -> Result<Self, RuntimeError> {
        Ok(Self {
            registry_path: PathBuf::from(DEFAULT_REGISTRY),
            actor_uid: current_uid()?,
            systemd: SystemdClient::new(SYSTEMD_TIMEOUT),
        })
    }

    pub(crate) fn list(&self) -> Result<String, RuntimeError> {
        let registry = Registry::load(&self.registry_path)?;
        let mut output = String::new();
        for application in registry
            .applications
            .iter()
            .filter(|application| self.actor_uid == 0 || application.uid == self.actor_uid)
        {
            output.push_str(&application.id);
            output.push('\n');
        }
        Ok(output)
    }

    pub(crate) fn run_action(
        &self,
        action: Action,
        application_id: &str,
    ) -> Result<String, RuntimeError> {
        let registry = Registry::load(&self.registry_path)?;
        let application = registry.resolve(application_id, self.actor_uid, action)?;
        let status = match action {
            Action::Status | Action::Logs => self.systemd.status(&application.unit)?,
            Action::Start | Action::Stop | Action::Restart => {
                self.systemd.lifecycle(action, &application.unit)?
            }
        };

        Ok(format!(
            "application: {}\ntenant: {}\nuser: {}\ngroup: {}\nunit-state: {}\nactive-state: {}\nsub-state: {}\nmain-pid: {}\n",
            application.id,
            application.tenant,
            application.user,
            application.group,
            status.load_state,
            status.active_state,
            status.sub_state,
            status.main_pid,
        ))
    }
}

fn current_uid() -> Result<u32, RuntimeError> {
    let status = std::fs::read_to_string("/proc/self/status")
        .map_err(|error| RuntimeError::Actor(error.to_string()))?;
    let uid_line = status
        .lines()
        .find(|line| line.starts_with("Uid:"))
        .ok_or_else(|| RuntimeError::Actor("/proc/self/status has no Uid entry".to_owned()))?;
    uid_line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| RuntimeError::Actor("Uid entry has no effective UID".to_owned()))?
        .parse()
        .map_err(|error| RuntimeError::Actor(format!("invalid effective UID: {error}")))
}
