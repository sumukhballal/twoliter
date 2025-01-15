use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs::{File, Permissions};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use tempfile::TempDir;

const COMPRESSED_KRANE_BIN: &[u8] = include_bytes!(env!("KRANE_GZ_PATH"));

lazy_static::lazy_static! {
    pub static ref KRANE: Krane = Krane::seal().unwrap();
}

#[derive(Debug)]
pub struct Krane {
    // Hold the file in memory to keep the fd open
    _tmp_dir: TempDir,
    path: PathBuf,
}

impl Krane {
    fn seal() -> Result<Krane> {
        let tmp_dir = TempDir::new()?;
        let path = tmp_dir.path().join("krane");

        let mut krane_file = File::create(&path)?;
        let permissions = Permissions::from_mode(0o755);
        krane_file.set_permissions(permissions)?;

        let mut krane_reader = GzDecoder::new(COMPRESSED_KRANE_BIN);

        std::io::copy(&mut krane_reader, &mut krane_file)?;

        Ok(Krane {
            _tmp_dir: tmp_dir,
            path,
        })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_krane_runs() {
        let status = Command::new(KRANE.path())
            .arg("--help")
            .output()
            .expect("failed to run krane");

        assert_eq!(status.status.code().unwrap(), 0);
    }
}
