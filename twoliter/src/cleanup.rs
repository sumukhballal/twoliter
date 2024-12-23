//! Provides a mechanism for cleaning up resources when twoliter is interrupted.

use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tempfile::TempPath;
use uuid::Uuid;

use self::sealed::*;

lazy_static::lazy_static! {
    pub(crate) static ref JANITOR: TempfileJanitor = TempfileJanitor::default();
}

impl TempfileJanitor {
    /// Run a given async closure using a [`tempfile::TempPath`].
    ///
    /// The closure has access to the path where the (closed) tempfile is stored.
    /// [`TempfileJanitor`] will ensure that the temporary file is deleted in the case that the
    /// current process receives SIGINT/SIGTERM/SIGHUP.
    pub(crate) async fn with_tempfile<R, Fut>(
        &self,
        tmpfile: TempPath,
        do_: impl FnOnce(PathBuf) -> Fut,
    ) -> Result<R>
    where
        Fut: Future<Output = R>,
    {
        let path = tmpfile.to_path_buf();
        let path_id = Uuid::new_v4();

        self.paths.lock().unwrap().insert(path_id, tmpfile);

        let result = do_(path).await;

        self.paths.lock().unwrap().remove(&path_id);

        Ok(result)
    }

    pub(crate) fn try_cleanup(&mut self) {
        tracing::info!("Cleaning up temporary resources...");
        if let Ok(mut paths) = self.paths.lock() {
            while let Some((_, path)) = paths.pop_first() {
                tracing::debug!("Deleting tempfile at '{}'", path.display());
                if let Err(e) = std::fs::remove_file(&path) {
                    tracing::error!("Failed to clean tempfile '{}': {}", path.display(), e);
                }
            }
        }
        tracing::info!("Done cleaning up.");
    }

    /// Attempts to install the cleanup process as a SIGINT/SIGTERM/SIGHUP signal handler
    pub(crate) fn setup_signal_handler(&self) -> Result<()> {
        let mut handler_ref = Self {
            paths: Arc::clone(&self.paths),
        };

        let already_handling = Arc::new(AtomicBool::new(false));
        ctrlc::try_set_handler(move || {
            if already_handling
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
            {
                handler_ref.try_cleanup();
            }
            // SIGINT is 130
            std::process::exit(130);
        })
        .context("Failed to create cleanup signal handler")
    }
}

/// Signal handlers are global -- hide `TempfileJanitor` to encourage use of the static reference.
mod sealed {
    use super::*;

    #[derive(Default, Debug)]
    pub(crate) struct TempfileJanitor {
        pub(super) paths: Arc<Mutex<BTreeMap<Uuid, TempPath>>>,
    }
}
