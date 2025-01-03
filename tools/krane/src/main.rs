use anyhow::{Context, Result};
use krane_static::call_krane_inherited_io;

fn main() -> Result<()> {
    let krane_args = &std::env::args().collect::<Vec<_>>()[1..];
    let krane_status = call_krane_inherited_io(krane_args)
        .with_context(|| format!("Failed to run krane with args {krane_args:?}"))?;

    anyhow::ensure!(krane_status.success(), "Krane exited with non-zero status");
    Ok(())
}
