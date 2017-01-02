//! Error type and helper functions.

use std::io;
use std::num;
use std::fmt;
use std::str;

use std::error::Error;
use std::process::ExitStatus;
use std::os::unix::process::ExitStatusExt;

use nix;
use nix::sys::signal::Signal;

#[derive(Debug)]
pub enum HLError {
    UnsuccessfulChild { status: String, cmdline: String },
    IOError           { cause: io::Error, detail: String },
    NixError          { cause: nix::Error, detail: String },
    PIError           { cause: num::ParseIntError, detail: String },
    UTF8Error         { cause: str::Utf8Error, detail: String },
}

impl fmt::Display for HLError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &HLError::UnsuccessfulChild { ref status, ref cmdline } => {
                write!(f, "Child process '{}' {}.", cmdline, status)
            },
            &HLError::IOError { ref cause, ref detail } => {
                write!(f, "{}: {}.", detail, cause)
            },
            &HLError::NixError { ref cause, ref detail } => {
                write!(f, "{}: {}.", detail, cause)
            },
            &HLError::PIError { ref cause, ref detail } => {
                write!(f, "Invalid integer {}: {}.", detail, cause)
            },
            &HLError::UTF8Error { ref cause, ref detail } => {
                write!(f, "Invalid UTF-8 in {}: {}.", detail, cause)
            }
        }
    }
}

impl Error for HLError {
    fn description(&self) -> &'static str {
        match self {
            &HLError::UnsuccessfulChild { .. } => "Child process failed",
            &HLError::IOError           { .. } => "I/O error",
            &HLError::NixError          { .. } => "System error",
            &HLError::PIError           { .. } => "Invalid integer",
            &HLError::UTF8Error         { .. } => "Invalid UTF-8 text",
        }
    }
    fn cause(&self) -> Option<&Error> {
        match self {
            &HLError::UnsuccessfulChild { .. } => None,
            &HLError::IOError           { ref cause, .. } => Some(cause),
            &HLError::NixError          { ref cause, .. } => Some(cause),
            &HLError::PIError           { ref cause, .. } => Some(cause),
            &HLError::UTF8Error         { ref cause, .. } => Some(cause),
        }
    }
}

pub fn map_unsuc_child (status: &ExitStatus, cmdline: &[&str]) -> HLError {
    let status = match status.code() {
        Some(n) => format!("exited unsuccessfully (code {})", n),
        None => match status.signal() {
            Some(n) => {
                // Neither nix nor libc exposes strsignal(), feh.
                // This is better than printing the raw signal number.
                if let Ok(sig) = Signal::from_c_int(n) {
                    format!("killed by {:?}", sig)
                } else {
                    format!("killed by signal {}", n)
                }
            }
            None => unreachable!(),
        }
    };
    // FIXME: shell-quote as necessary.
    let cmd = cmdline.join(" ");
    HLError::UnsuccessfulChild { status: status, cmdline: cmd }
}

pub fn map_io_err (cause: io::Error, detail: String) -> HLError {
    HLError::IOError { cause: cause, detail: detail }
}
pub fn map_nix_err (cause: nix::Error, detail: String) -> HLError {
    HLError::NixError { cause: cause, detail: detail }
}
pub fn map_pi_err (cause: num::ParseIntError, detail: String) -> HLError {
    HLError::PIError { cause: cause, detail: detail }
}
pub fn map_utf8_err (cause: str::Utf8Error, detail: String) -> HLError {
    HLError::UTF8Error { cause: cause, detail: detail }
}
