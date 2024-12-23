//! Rust interface to the golang FFI bindings for krane.
use anyhow::{ensure, Context, Result};
use std::ffi::{c_char, CStr, CString};
use std::os::unix::process::ExitStatusExt;
use std::process::ExitStatus;
use std::ptr;

pub type KraneError = anyhow::Error;

/// Calls the go-containerregistry `krane` binary.
///
/// The function is a thin wrapper around a statically compiled version of the binary which is
/// called via the C FFI.
pub fn call_krane(args: &[impl AsRef<str>]) -> Result<std::process::Output> {
    let argv_owned = c_args(args)?;

    let mut argv: Vec<*mut c_char> = argv_owned
        .iter()
        .map(|arg| arg.as_ptr() as *mut c_char)
        .collect();
    let argc = argv.len() as i32;

    // stdout/stderr are written to buffers, to which these will eventually point
    let mut c_stdout: *mut c_char = ptr::null_mut();
    let mut c_stderr: *mut c_char = ptr::null_mut();

    let status_code =
        unsafe { extern_krane::krane(argc, argv.as_mut_ptr(), &mut c_stdout, &mut c_stderr) };

    let c_stdout = CStringBuffer(c_stdout);
    let c_stderr = CStringBuffer(c_stderr);

    let stdout = c_stdout
        .try_into()
        .context("krane ffi returned null pointer for stdout")?;

    let stderr = c_stderr
        .try_into()
        .context("krane ffi returned null pointer for stderr")?;

    Ok(std::process::Output {
        stdout,
        stderr,
        status: ExitStatus::from_raw(status_code),
    })
}

/// Calls the go-containerregistry `krane` binary.
///
/// Unlike `call_krane`, output goes directly to stdout/stderr.
pub fn call_krane_inherited_io(args: &[impl AsRef<str>]) -> Result<ExitStatus> {
    let argv_owned = c_args(args)?;

    let mut argv: Vec<*mut c_char> = argv_owned
        .iter()
        .map(|arg| arg.as_ptr() as *mut c_char)
        .collect();
    let argc = argv.len() as i32;

    let status_code = unsafe { extern_krane::krane_inherited_io(argc, argv.as_mut_ptr()) };

    Ok(ExitStatus::from_raw(status_code))
}

fn c_args(args: &[impl AsRef<str>]) -> Result<Vec<CString>> {
    args.iter()
        .map(|arg| {
            CString::new(arg.as_ref().as_bytes())
                .map_err(|_| anyhow::Error::msg("krane args contained illegal null byte"))
        })
        .collect::<Result<Vec<_>>>()
}

/// Wrapper type around null-terminated C Strings representing arbitrary byte buffers.
///
/// frees the wrapped pointer on drop.
struct CStringBuffer(*mut c_char);

impl Drop for CStringBuffer {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { libc::free(self.0 as *mut std::ffi::c_void) };
        }
    }
}

impl TryInto<Vec<u8>> for CStringBuffer {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<Vec<u8>> {
        ensure!(!self.0.is_null(), "FFI returned null pointer");
        Ok(unsafe { CStr::from_ptr(self.0).to_bytes().to_vec() })
    }
}

mod extern_krane {
    use std::os::raw::{c_char, c_int};

    extern "C" {
        pub(crate) fn krane(
            argc: c_int,
            argv: *mut *mut c_char,
            stdout: *mut *mut c_char,
            stderr: *mut *mut c_char,
        ) -> c_int;

        pub(crate) fn krane_inherited_io(argc: c_int, argv: *mut *mut c_char) -> c_int;

    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_krane_runs() {
        let krane_res = call_krane(&["--help"]).unwrap();
        println!("{:?}", krane_res);

        assert!(krane_res.status.success());
        assert!(String::from_utf8_lossy(&krane_res.stdout).starts_with("krane"));
    }

    #[test]
    fn test_krane_inherited_io_doesnt_explode() {
        call_krane_inherited_io(&["--help"]).unwrap();
    }
}
