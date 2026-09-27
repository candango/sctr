use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

const REGISTRY_ROOT: &str = "/opt/home";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Start,
    Stop,
    Restart,
    Status,
    Logs,
}

impl Action {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Stop => "stop",
            Self::Restart => "restart",
            Self::Status => "status",
            Self::Logs => "logs",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct Registry {
    pub applications: Vec<Application>,
}

#[derive(Debug, Deserialize)]
pub struct Application {
    pub id: String,
    pub tenant: String,
    pub uid: u32,
    pub gid: u32,
    pub user: String,
    pub group: String,
    pub unit: String,
    pub root: PathBuf,
    pub actions: Vec<Action>,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("cannot read registry {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("cannot parse registry {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("registry entry is invalid: {0}")]
    Invalid(String),
    #[error("application {0:?} is not registered")]
    UnknownApplication(String),
    #[error("application {application:?} is not owned by the current actor")]
    Unauthorized { application: String },
    #[error("action {action:?} is not allowed for application {application:?}")]
    ActionDenied { application: String, action: String },
}

impl Registry {
    pub fn load(path: &Path) -> Result<Self, RegistryError> {
        let contents = std::fs::read_to_string(path).map_err(|source| RegistryError::Read {
            path: path.to_owned(),
            source,
        })?;
        let registry: Self =
            serde_json::from_str(&contents).map_err(|source| RegistryError::Parse {
                path: path.to_owned(),
                source,
            })?;
        registry.validate()?;
        Ok(registry)
    }

    pub(crate) fn resolve(
        &self,
        application_id: &str,
        actor_uid: u32,
        action: Action,
    ) -> Result<&Application, RegistryError> {
        let application = self
            .applications
            .iter()
            .find(|application| application.id == application_id)
            .ok_or_else(|| RegistryError::UnknownApplication(application_id.to_owned()))?;

        if actor_uid != 0 && actor_uid != application.uid {
            return Err(RegistryError::Unauthorized {
                application: application_id.to_owned(),
            });
        }

        if !application.actions.contains(&action) {
            return Err(RegistryError::ActionDenied {
                application: application_id.to_owned(),
                action: action.as_str().to_owned(),
            });
        }

        Ok(application)
    }

    fn validate(&self) -> Result<(), RegistryError> {
        if self.applications.is_empty() {
            return Err(RegistryError::Invalid(
                "applications must not be empty".to_owned(),
            ));
        }

        for (index, application) in self.applications.iter().enumerate() {
            if !is_identifier(&application.id) {
                return Err(RegistryError::Invalid(format!(
                    "applications[{index}].id must be a lowercase identifier"
                )));
            }
            if application.tenant.is_empty()
                || application.user.is_empty()
                || application.group.is_empty()
            {
                return Err(RegistryError::Invalid(format!(
                    "applications[{index}] must define tenant, user, and group"
                )));
            }
            if application.uid == 0 || application.gid == 0 {
                return Err(RegistryError::Invalid(format!(
                    "applications[{index}] cannot run with a privileged identity"
                )));
            }
            if !valid_unit_name(&application.unit) {
                return Err(RegistryError::Invalid(format!(
                    "applications[{index}].unit is not an SCTR service unit"
                )));
            }
            if !application.root.is_absolute()
                || !application.root.starts_with(REGISTRY_ROOT)
                || application.root == Path::new(REGISTRY_ROOT)
            {
                return Err(RegistryError::Invalid(format!(
                    "applications[{index}].root must be below {REGISTRY_ROOT}"
                )));
            }
            if application.actions.is_empty() {
                return Err(RegistryError::Invalid(format!(
                    "applications[{index}].actions must not be empty"
                )));
            }
        }

        for (index, left) in self.applications.iter().enumerate() {
            if self.applications[index + 1..]
                .iter()
                .any(|right| right.id == left.id || right.unit == left.unit)
            {
                return Err(RegistryError::Invalid(format!(
                    "applications contains duplicate id or unit at index {index}"
                )));
            }
        }

        Ok(())
    }
}

fn is_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
}

fn valid_unit_name(value: &str) -> bool {
    value.starts_with("sctr-")
        && value.ends_with(".service")
        && !value.contains('/')
        && !value.contains("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> Registry {
        serde_json::from_str(
            r#"{
                "applications": [{
                    "id": "api",
                    "tenant": "projenv",
                    "uid": 2000,
                    "gid": 2000,
                    "user": "projenv",
                    "group": "projenv",
                    "unit": "sctr-projenv-api.service",
                    "root": "/opt/home/projenv/apps/api",
                    "actions": ["start", "status"]
                }]
            }"#,
        )
        .expect("test registry is valid")
    }

    #[test]
    fn resolves_only_the_registered_tenant_and_action() {
        let registry = registry();

        assert!(registry.resolve("api", 2000, Action::Status).is_ok());
        assert!(matches!(
            registry.resolve("api", 3003, Action::Status),
            Err(RegistryError::Unauthorized { .. })
        ));
        assert!(matches!(
            registry.resolve("api", 2000, Action::Stop),
            Err(RegistryError::ActionDenied { .. })
        ));
        assert!(registry.resolve("api", 0, Action::Start).is_ok());
    }

    #[test]
    fn rejects_privileged_and_unscoped_registry_entries() {
        let mut registry = registry();
        registry.applications[0].uid = 0;
        assert!(registry.validate().is_err());

        registry.applications[0].uid = 2000;
        registry.applications[0].root = PathBuf::from("/tmp/api");
        assert!(registry.validate().is_err());
    }
}
