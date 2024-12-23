use std::fmt::Debug;
use std::fs::File;
use std::path::Path;

use async_trait::async_trait;
use krane_static::{call_krane, call_krane_inherited_io};
use snafu::{ensure, ResultExt};
use tar::Archive as TarArchive;
use tempfile::TempDir;

use crate::{error, ConfigView, DockerArchitecture, ImageToolImpl, ImageView, Result};

#[derive(Debug)]
pub struct CraneCLI;

impl CraneCLI {
    /// Enables verbose logging of crane if debug logging is enabled.
    fn crane_cmd<'a>(cmd: &[&'a str]) -> Vec<&'a str> {
        if log::max_level() >= log::LevelFilter::Debug {
            [&["-v"], cmd].concat()
        } else {
            cmd.into()
        }
    }

    fn debug_cmd(args: &[&str]) -> String {
        [
            vec!["krane".to_string()],
            args.iter()
                .map(|arg| format!("'{}'", arg))
                .collect::<Vec<_>>(),
        ]
        .concat()
        .join(", ")
    }

    /// Calls `krane` with the given arguments.
    ///
    /// Returns stdout if the process successfully completes.
    async fn output(cmd: &[&str], error_msg: &str) -> Result<Vec<u8>> {
        let args = Self::crane_cmd(cmd);

        log::debug!("Executing [{}]", Self::debug_cmd(cmd));

        let fork_args = args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let output = tokio::task::spawn_blocking(move || call_krane(&fork_args))
            .await
            .context(error::ForkSnafu)?
            .context(error::CraneFFISnafu)?;

        log::debug!(
            "[{}] stdout: {}",
            Self::debug_cmd(&args),
            String::from_utf8_lossy(&output.stdout).to_string()
        );
        log::debug!(
            "[{}] stderr: {}",
            Self::debug_cmd(&args),
            String::from_utf8_lossy(&output.stderr).to_string()
        );

        ensure!(
            output.status.success(),
            error::OperationFailedSnafu {
                message: error_msg,
                program: "krane",
                args: args.iter().map(|x| x.to_string()).collect::<Vec<_>>()
            }
        );

        Ok(output.stdout)
    }

    /// Calls `krane` with the given arguments.
    ///
    /// stdout/stderr is inherited from the current process.
    async fn call(cmd: &[&str], error_msg: &str) -> Result<()> {
        let args = Self::crane_cmd(cmd);

        log::debug!("Executing [{}]", Self::debug_cmd(cmd));

        let fork_args = args.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let status = tokio::task::spawn_blocking(move || call_krane_inherited_io(&fork_args))
            .await
            .context(error::ForkSnafu)?
            .context(error::CraneFFISnafu)?;

        ensure!(
            status.success(),
            error::OperationFailedSnafu {
                message: error_msg,
                program: "krane",
                args: args.iter().map(|x| x.to_string()).collect::<Vec<_>>()
            }
        );

        Ok(())
    }
}

#[async_trait]
impl ImageToolImpl for CraneCLI {
    async fn pull_oci_image(&self, path: &Path, uri: &str) -> Result<()> {
        let archive_path = path.to_string_lossy();
        Self::call(
            &["pull", "--format", "oci", uri, archive_path.as_ref()],
            &format!("failed to pull image archive from {}", uri),
        )
        .await
    }

    async fn get_manifest(&self, uri: &str) -> Result<Vec<u8>> {
        Self::output(
            &["manifest", uri],
            &format!("failed to fetch manifest for resource at {}", uri),
        )
        .await
    }

    async fn get_config(&self, uri: &str) -> Result<ConfigView> {
        let bytes = Self::output(
            &["config", uri],
            &format!("failed to fetch image config from {}", uri),
        )
        .await?;
        let image_view: ImageView =
            serde_json::from_slice(bytes.as_slice()).context(error::ConfigDeserializeSnafu)?;
        Ok(image_view.config)
    }

    async fn push_oci_archive(&self, path: &Path, uri: &str) -> Result<()> {
        let temp_dir = TempDir::new_in(path.parent().unwrap()).context(error::CraneTempSnafu)?;

        let mut oci_file = File::open(path).context(error::ArchiveReadSnafu)?;

        let mut oci_archive = TarArchive::new(&mut oci_file);
        oci_archive
            .unpack(temp_dir.path())
            .context(error::ArchiveExtractSnafu)?;
        Self::call(
            &["push", &temp_dir.path().to_string_lossy(), uri],
            &format!("failed to push image {}", uri),
        )
        .await
    }

    async fn push_multi_platform_manifest(
        &self,
        platform_images: Vec<(DockerArchitecture, String)>,
        uri: &str,
    ) -> Result<()> {
        let images: Vec<&str> = platform_images
            .iter()
            .map(|(_, image)| image.as_str())
            .collect();

        let mut manifest_create_args = vec!["index", "append"];
        for image in images {
            manifest_create_args.extend_from_slice(&["-m", image])
        }
        manifest_create_args.extend_from_slice(&["-t", uri]);
        Self::call(
            &manifest_create_args,
            &format!("could not push multi-platform manifest to {}", uri),
        )
        .await
    }
}
