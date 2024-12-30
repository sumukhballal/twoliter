//! This module defines common atomic build tasks that can be performed with a fully loaded project.
use super::{LockedSDKProvider, Project};
use crate::cleanup::JANITOR;
use crate::docker::Docker;
use anyhow::{Context, Result};
use krane_static::call_krane_inherited_io;
use tracing::instrument;

impl<T: LockedSDKProvider> Project<T> {
    /// Caches the project's SDK into the docker daemon if an image with the same name/tag is not
    /// already cached.
    #[instrument(level = "trace")]
    pub(crate) async fn fetch_sdk(&self) -> Result<()> {
        let sdk_uri = self.sdk_image().project_image_uri();
        tracing::info!("Ensuring project SDK '{sdk_uri}' is cached locally.");

        if Docker::image_is_cached(&sdk_uri).await? {
            tracing::debug!("SDK '{sdk_uri}' is cached.");
            return Ok(());
        }

        let sdk_archive_dir = self.external_sdk_archive_dir();
        tokio::fs::create_dir_all(&sdk_archive_dir).await?;

        let temp_path = tempfile::Builder::new()
            .prefix("bottlerocket-sdk-tmp-ardchive-")
            .suffix(".tar")
            .tempfile_in(&sdk_archive_dir)?
            .into_temp_path();

        let host_platform = Docker::host_platform().await?;

        JANITOR
            .with_tempfile(temp_path, |temp_path| async move {
                let path_str = temp_path.to_string_lossy().to_string();

                tracing::info!("Pulling '{sdk_uri}' for platform '{host_platform}'");
                call_krane_inherited_io(&[
                    "pull",
                    &sdk_uri.uri(),
                    &path_str,
                    "--platform",
                    &host_platform,
                ])
                .context("Failed to pull SDK image")?;

                tracing::info!("Loading SDK image '{sdk_uri}' into docker daemon");
                Docker::load(&path_str).await?;

                Ok::<_, anyhow::Error>(())
            })
            .await??;

        Ok(())
    }
}
