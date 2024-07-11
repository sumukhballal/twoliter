//! ImageTool enablement library implements a standardized way of calling commandline container image
//! tools for interacting primarily with kit images in a container registry.
//!
//! Current two tools are supported:
//! * crane, gcrane, krane
//!     Crane provides a more direct interaction with the container registry,
//!     allowing us to query image information in the registry without having to pull the full image to
//!     disk. It also does not require a daemon to operate and has optimizations for pulling large images to disk
//! * docker
//!     Docker can perform all interactions we need with several caveats that make it less efficient than
//!     crane. The image needs to be pulled locally in order for docker to inspect the manifest and extract
//!     metadata. In addition, in order to operate with OCI image format, the containerd-snapshotter
//!     feature has to be enabled in the docker daemon
use std::fmt::{Display, Formatter};
use std::{collections::HashMap, env, path::Path, rc::Rc};

use async_trait::async_trait;
use cli::CommandLine;
use crane::CraneCLI;
use docker::DockerCLI;
use serde::Deserialize;
use snafu::ResultExt;
use which::which;

mod cli;
mod crane;
mod docker;

#[async_trait]
pub trait ImageTool {
    /// Pull an image archive to disk
    async fn pull_oci_image(&self, path: &Path, uri: &str) -> Result<()>;
    /// Fetch the image config
    async fn get_config(&self, uri: &str) -> Result<ConfigView>;
    /// Fetch the manifest
    async fn get_manifest(&self, uri: &str) -> Result<Vec<u8>>;
    /// Push a single-arch image in oci archive format
    async fn push_oci_archive(&self, path: &Path, uri: &str) -> Result<()>;
    /// Push the multi-arch kit manifest list
    async fn push_multi_platform_manifest(
        &self,
        platform_images: Vec<(DockerArchitecture, String)>,
        uri: &str,
    ) -> Result<()>;
}

/// Auto-select the container tool to use by environment variable
/// and-or auto detection
pub fn image_tool() -> Result<Rc<dyn ImageTool>> {
    if let Ok(name) = env::var("TWOLITER_KIT_IMAGE_TOOL") {
        return match name.as_str() {
            "docker" => Ok(Rc::new(DockerCLI {
                cli: CommandLine {
                    path: which("docker").context(error::NotFoundSnafu { name: "docker" })?,
                },
            })),
            tool @ ("crane" | "gcrane" | "krane") => Ok(Rc::new(CraneCLI {
                cli: CommandLine {
                    path: which(tool).context(error::NotFoundSnafu { name: tool })?,
                },
            })),
            _ => error::UnsupportedSnafu { name }.fail(),
        };
    }
    let crane = which("krane").or(which("gcrane")).or(which("crane"));
    if let Ok(path) = crane {
        return Ok(Rc::new(CraneCLI {
            cli: CommandLine { path },
        }));
    };
    Ok(Rc::new(DockerCLI {
        cli: CommandLine {
            path: which("docker").context(error::NoneFoundSnafu)?,
        },
    }))
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum DockerArchitecture {
    Amd64,
    Arm64,
}

impl TryFrom<&str> for DockerArchitecture {
    type Error = error::Error;

    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "x86_64" | "amd64" => Ok(DockerArchitecture::Amd64),
            "aarch64" | "arm64" => Ok(DockerArchitecture::Arm64),
            _ => Err(error::Error::InvalidArchitecture {
                value: value.to_string(),
            }),
        }
    }
}

impl Display for DockerArchitecture {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Amd64 => "amd64",
            Self::Arm64 => "arm64",
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
struct ImageView {
    config: ConfigView,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct ConfigView {
    pub labels: HashMap<String, String>,
}

pub type Result<T> = std::result::Result<T, error::Error>;

pub mod error {
    use std::path::PathBuf;

    use snafu::Snafu;

    #[derive(Snafu, Debug)]
    #[snafu(visibility(pub(super)))]
    pub enum Error {
        #[snafu(display("Failed to extract archive: {source}"))]
        ArchiveExtract { source: std::io::Error },

        #[snafu(display("Failed to read archive: {source}"))]
        ArchiveRead { source: std::io::Error },

        #[snafu(display("Failed to execute image tool, {message}: {source}"))]
        CommandFailed {
            message: String,
            source: std::io::Error,
        },

        #[snafu(display("Failed to deserialize image config: {source}"))]
        ConfigDeserialize { source: serde_json::Error },

        #[snafu(display("Failed to create temporary directory for crane push: {source}"))]
        CraneTemp { source: std::io::Error },

        #[snafu(display("Failed to create temporary directory for docker save: {source}"))]
        DockerTemp { source: std::io::Error },

        #[snafu(display("invalid architecture '{value}'"))]
        InvalidArchitecture { value: String },

        #[snafu(display("No digest returned by `docker load`"))]
        NoDigest,

        #[snafu(display(
            "Unable to find any supported container image tool, please install docker or crane: {}",
            source
        ))]
        NoneFound { source: which::Error },

        #[snafu(display(
            "Unable to find a container image tool by name '{}' in current environment",
            name
        ))]
        NotFound { name: String, source: which::Error },

        #[snafu(display("Failed to run operation with image tool: {message}\n command: {} {}", program.display(), args.join(" ")))]
        OperationFailed {
            message: String,
            program: PathBuf,
            args: Vec<String>,
        },

        #[snafu(display("Failed to parse kit filename: {}", source))]
        Regex { source: regex::Error },

        #[snafu(display("Unsupported container image tool '{}'", name))]
        Unsupported { name: String },
    }
}