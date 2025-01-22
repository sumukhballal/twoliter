//! This module performs checks that the current environment is compatible with twoliter, as well
//! as any other "global" setup that must occur before the build process begins.
use anyhow::{ensure, Result};
use which::which_global;

const REQUIRED_TOOLS: &[&str] = &["docker", "gzip", "lz4"];

/// Runs all common setup required for twoliter.
///
/// * Ensures that any required system tools are installed an accessible.
pub(crate) async fn preflight() -> Result<()> {
    check_environment().await?;

    Ok(())
}

pub(crate) async fn check_environment() -> Result<()> {
    check_for_required_tools()?;

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
