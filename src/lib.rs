//! Shared code among all three of the programs.
//! This is technically its own "crate".  I think.

#![cfg(unix)]
//#![feature(process_exec)]

extern crate nix;
extern crate libc;

pub use libc::pid_t;

mod err;
pub use err::*;

mod subprocess;
pub use subprocess::*;

mod idle_loop;
pub use idle_loop::*;
