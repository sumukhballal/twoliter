//! This module performs checks that the current environment is compatible with twoliter, as well
//! as any other "global" setup that must occur before the build process begins.
use anyhow::{ensure, Result};
use lazy_static::lazy_static;
use semver::{Comparator, Op, Prerelease, VersionReq};
use which::which_global;

use crate::docker::Docker;

const REQUIRED_TOOLS: &[&str] = &["docker", "gzip", "lz4"];

lazy_static! {
    // Twoliter relies on minimum Dockerfile syntax 1.4.3, which is shipped in Docker 23.0.0 by default
    // We do not use explicit `syntax=` directives to avoid network connections during the build.
    static ref MINIMUM_DOCKER_VERSION: VersionReq = VersionReq {
        comparators: [
            Comparator {
                op: Op::GreaterEq,
                major: 23,
                minor: None,
                patch: None,
                pre: Prerelease::default(),
            }
        ].into()
    };
}

/// Runs all common setup required for twoliter.
///
/// * Ensures that any required system tools are installed an accessible.
/// * Sets up interrupt handler to cleanup on SIGINT
pub(crate) async fn preflight() -> Result<()> {
    check_environment().await?;

    Ok(())
}

pub(crate) async fn check_environment() -> Result<()> {
    check_for_required_tools()?;
    check_docker_version().await?;

    Ok(())
}

fn check_for_required_tools() -> Result<()> {
    for tool in REQUIRED_TOOLS {
        ensure!(
            which_global(tool).is_ok(),
            "Failed to find required tool `{tool}` in PATH"
        );
    }
    Ok(())
}

async fn check_docker_version() -> Result<()> {
    let docker_version = Docker::server_version().await?;

    ensure!(
        MINIMUM_DOCKER_VERSION.matches(&docker_version),
        "docker found in PATH does not meet the minimum version requirements for twoliter: {}",
        MINIMUM_DOCKER_VERSION.to_string(),
    );

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use semver::Version;
    use test_case::test_case;

    #[test_case(Version::parse("25.0.5").unwrap(), true; "25.0.5 passes")]
    #[test_case(Version::parse("27.1.4").unwrap(), true; "27.1.4 passes")]
    #[test_case(Version::parse("18.0.9").unwrap(), false; "18.0.9 fails")]
    #[test_case(Version::parse("20.10.27").unwrap(), false)]
    fn test_docker_version_req(version: Version, is_ok: bool) {
        assert_eq!(MINIMUM_DOCKER_VERSION.matches(&version), is_ok)
    }
}
