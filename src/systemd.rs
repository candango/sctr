use std::time::Duration;

use futures_lite::{StreamExt, future};
use thiserror::Error;
use zbus::{
    Connection, Proxy,
    zvariant::{OwnedObjectPath, OwnedValue},
};

use crate::registry::Action;

const SYSTEMD_SERVICE: &str = "org.freedesktop.systemd1";
const MANAGER_PATH: &str = "/org/freedesktop/systemd1";
const MANAGER_INTERFACE: &str = "org.freedesktop.systemd1.Manager";
const PROPERTIES_INTERFACE: &str = "org.freedesktop.DBus.Properties";
const UNIT_INTERFACE: &str = "org.freedesktop.systemd1.Unit";
const SERVICE_INTERFACE: &str = "org.freedesktop.systemd1.Service";

#[derive(Debug, Error)]
pub enum SystemdError {
    #[error("systemd D-Bus error: {0}")]
    Dbus(#[from] zbus::Error),
    #[error("systemd job for {unit:?} failed with result {result:?}")]
    JobFailed { unit: String, result: String },
    #[error("timed out waiting for systemd job for {unit:?}")]
    JobTimeout { unit: String },
    #[error("systemd closed the job signal stream before completing {unit:?}")]
    JobSignalClosed { unit: String },
    #[error("cannot decode systemd property {property:?}: {reason}")]
    Property { property: String, reason: String },
}

#[derive(Debug, Eq, PartialEq)]
pub struct UnitStatus {
    pub load_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub main_pid: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct SystemdClient {
    timeout: Duration,
}

impl SystemdClient {
    pub(crate) fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub(crate) fn status(&self, unit: &str) -> Result<UnitStatus, SystemdError> {
        future::block_on(self.status_async(unit))
    }

    pub(crate) fn lifecycle(&self, action: Action, unit: &str) -> Result<UnitStatus, SystemdError> {
        future::block_on(self.lifecycle_async(action, unit))
    }

    async fn status_async(&self, unit: &str) -> Result<UnitStatus, SystemdError> {
        let connection = self.connection().await?;
        let manager = Proxy::new(
            &connection,
            SYSTEMD_SERVICE,
            MANAGER_PATH,
            MANAGER_INTERFACE,
        )
        .await?;
        let unit_path: OwnedObjectPath = manager.call("LoadUnit", &(unit,)).await?;
        let properties = Proxy::new(
            &connection,
            SYSTEMD_SERVICE,
            unit_path.as_str(),
            PROPERTIES_INTERFACE,
        )
        .await?;

        Ok(UnitStatus {
            load_state: property_string(&properties, "LoadState").await?,
            active_state: property_string(&properties, "ActiveState").await?,
            sub_state: property_string(&properties, "SubState").await?,
            main_pid: property_u32(&properties, SERVICE_INTERFACE, "MainPID").await?,
        })
    }

    async fn lifecycle_async(
        &self,
        action: Action,
        unit: &str,
    ) -> Result<UnitStatus, SystemdError> {
        let method = match action {
            Action::Start => "StartUnit",
            Action::Stop => "StopUnit",
            Action::Restart => "RestartUnit",
            Action::Status | Action::Logs => {
                return self.status_async(unit).await;
            }
        };

        let connection = self.connection().await?;
        let manager = Proxy::new(
            &connection,
            SYSTEMD_SERVICE,
            MANAGER_PATH,
            MANAGER_INTERFACE,
        )
        .await?;
        let mut jobs = manager.receive_signal("JobRemoved").await?;
        let job_path: OwnedObjectPath = manager.call(method, &(unit, "replace")).await?;

        let unit_name = unit.to_owned();
        let wait_for_job = async move {
            while let Some(message) = jobs.next().await {
                let (_id, path, job_unit, result): (u32, OwnedObjectPath, String, String) =
                    message.body().deserialize()?;
                if path == job_path && job_unit == unit_name {
                    return if result == "done" {
                        Ok(())
                    } else {
                        Err(SystemdError::JobFailed {
                            unit: job_unit,
                            result,
                        })
                    };
                }
            }
            Err(SystemdError::JobSignalClosed { unit: unit_name })
        };
        let timeout_unit = unit.to_owned();
        let timeout = async move {
            async_io::Timer::after(self.timeout).await;
            Err(SystemdError::JobTimeout { unit: timeout_unit })
        };

        future::race(wait_for_job, timeout).await?;
        self.status_async(unit).await
    }

    async fn connection(&self) -> Result<Connection, SystemdError> {
        Ok(zbus::connection::Builder::system()?
            .method_timeout(self.timeout)
            .build()
            .await?)
    }
}

async fn property_string(properties: &Proxy<'_>, property: &str) -> Result<String, SystemdError> {
    let value: OwnedValue = properties.call("Get", &(UNIT_INTERFACE, property)).await?;
    String::try_from(value).map_err(|error| SystemdError::Property {
        property: property.to_owned(),
        reason: error.to_string(),
    })
}

async fn property_u32(
    properties: &Proxy<'_>,
    interface: &str,
    property: &str,
) -> Result<u32, SystemdError> {
    let value: OwnedValue = properties.call("Get", &(interface, property)).await?;
    u32::try_from(value).map_err(|error| SystemdError::Property {
        property: property.to_owned(),
        reason: error.to_string(),
    })
}
