use crate::common::{exec, exec_log};
use anyhow::{Context, Result};
use semver::Version;
use std::path::Path;
use tokio::process::Command;

use super::ImageUri;

pub(crate) struct Docker;

impl Docker {
    /// Loads an image tarball into the docker daemon from the given path
    pub(crate) async fn load(path: impl AsRef<Path>) -> Result<()> {
        exec_log(
            Command::new("docker")
                .args(["load", "-i"])
                .arg(path.as_ref()),
        )
        .await
    }

    /// Returns whether or not the docker daemon has cached an image with the given URI locally
    pub(crate) async fn image_is_cached(image_uri: &ImageUri) -> Result<bool> {
        let image_hash = exec(
            Command::new("docker")
                .args(["images", "-q"])
                .arg(image_uri.uri()),
            true,
        )
        .await
        // Convert Result<Option<String>> to Option<String>
        .ok()
        .flatten()
        .map(|s| s.trim().to_string())
        .with_context(|| {
            format!(
                "Failed to search docker daemon for image '{}'",
                image_uri.uri()
            )
        })?;

        Ok(!image_hash.is_empty())
    }

    /// Fetches the host platform in the form $OS/$GOARCH, e.g. linux/arm64
    pub(crate) async fn host_platform() -> Result<String> {
        exec(
            Command::new("docker").args(["version", "--format", "{{.Server.Os}}/{{.Server.Arch}}"]),
            true,
        )
        .await
        // Convert Result<Option<String>> to Option<String>
        .ok()
        .flatten()
        .map(|s| s.trim().to_string())
        .context("Failed to fetch host platform from docker")
    }

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
