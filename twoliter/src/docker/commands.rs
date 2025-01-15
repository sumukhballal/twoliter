use crate::common::exec;
use anyhow::{Context, Result};
use semver::Version;
use tokio::process::Command;

pub(crate) struct Docker;

impl Docker {
    /// Fetches the version of the docker daemon
    pub(crate) async fn server_version() -> Result<Version> {
        let version_str = exec(
            Command::new("docker").args(["version", "--format", "{{.Server.Version}}"]),
            true,
        )
        .await
        // Convert Result<Option<String>> to Option<String>
        .ok()
        .flatten()
        .map(|s| s.trim().to_string())
        .context("Failed to fetch docker version")?;

        Version::parse(&version_str).context("Failed to parse docker version as semver")
    }
}
