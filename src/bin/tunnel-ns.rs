//! Establish network namespaces that will use tunnel devices as their
//! default routes.
//!
//! Copyright © 2015-2017 Zack Weinberg
//! Licensed under the Apache License, Version 2.0 (the "License");
//! you may not use this file except in compliance with the License.
//! You may obtain a copy of the License at
//! http://www.apache.org/licenses/LICENSE-2.0
//! There is NO WARRANTY.
//!
//!     tunnel-ns PREFIX N
//!
//! creates N network namespaces, imaginatively named PREFIX_ns0,
//! PREFIX_ns1, ... The loopback device in each namespace is brought
//! up, with the usual address.  /etc/netns directories for each
//! namespace are created.  No other setup is performed.  (The tunnel
//! interfaces are expected to be created on the fly by a program like
//! 'openvpn-netns', which see.  This is because (AFAICT) if you create
//! a persistent tunnel ahead of time, and put its interface side into
//! a namespace, it then becomes impossible for anything to reattach
//! to the device side.)
//!
//! This program expects to be run with both stdin and stdout connected
//! to pipes.  As it creates each namespace, it writes one line to its
//! stdout:
//!
//!   PREFIX_nsX <newline>
//!
//! After all namespaces have been created, stdout is closed.
//!
//! Anything written to stdin is read and discarded.  When stdin is
//! *closed*, however, all of the network namespaces are torn down
//! (killing any processes still in there, if necessary) and the
//! program exits.  This also happens on receipt of any catchable
//! signal whose default action is to terminate the process without
//! a core dump (e.g. SIGTERM, SIGHUP).
//!
//! Errors, if any, will be written to stderr.
//!
//! This program must be installed setuid root.  It expects the "ip"
//! utility to be available in a standard "bin" directory (see
//! prepare_child_env for the PATH setting used).  It makes
//! extensive use of Linux-specific network stack features.
//! A port to a different OS might well entail a complete rewrite.

use std::process;
use std::env;
use std::io;
use std::fs;

use std::ascii::AsciiExt;
use std::convert::From;
use std::io::Write;
use std::path::PathBuf;

extern crate nix;
#[macro_use] extern crate clap;

// The internal shared-code crate has this awkward name because
// I haven't figured out how to make it less awkward.
extern crate openvpn_netns_tools;
use openvpn_netns_tools::*;

/// RAII class which creates and removes an /etc/netns directory
/// for a namespace.
struct NsConfDir<'a> {
    path: PathBuf,
    env: &'a ChildEnv
}
impl<'a> NsConfDir<'a> {
    fn new(name: &str, env: &'a ChildEnv) -> Result<NsConfDir<'a>, HLError> {
        let mut path = PathBuf::new();
        path.push("/etc/netns");
        path.push(name);
        if env.verbose {
            writeln!(io::stderr(), "mkdir {:?}", &path).unwrap();
        }
        if !env.dryrun {
            try!(fs::create_dir_all(&path)
                 .map_err(|e| map_io_err(e, format!(
                     "mkdir {:?}", &path))));
        }

        Ok(NsConfDir { path: path, env: env })
    }
}
impl<'a> Drop for NsConfDir<'a> {
    fn drop (&mut self) {
        if self.env.verbose {
            writeln!(io::stderr(), "rm -rf {:?}", &self.path).unwrap();
        }
        if !self.env.dryrun {
            if let Err(e) = fs::remove_dir_all(&self.path) {
                writeln!(io::stderr(),
                         "warning: could not delete {:?}: {:?}",
                         &self.path, e).unwrap();
            }
        }
    }
}

/// RAII class which creates and destroys a network namespace and its
/// /etc/netns directory.
struct NetNs<'a> {
    name:     String,
    _confdir: NsConfDir<'a>,
    env:      &'a ChildEnv
}
impl<'a> NetNs<'a> {
    fn new(name: String, env: &'a ChildEnv) -> Result<NetNs<'a>, HLError> {
        let confdir = try!(NsConfDir::new(&name, env));
        try!(run(&["ip", "netns", "add", &name], env));

        // The loopback interface automatically exists in the namespace,
        // with the usual address and an appropriate routing table entry,
        // but it is not brought up automatically.  If this fails, we must
        // tear down the namespace manually; RAII is not yet in effect.
        if let Err(e) = run(&["ip", "netns", "exec", &name,
                              "ip", "link", "set", "dev", "lo", "up"],
                            env) {
            run_ignore_failure(&["ip", "netns", "del", &name], env);
            return Err(e);
        }


        Ok(NetNs { name: name, _confdir: confdir, env: env })
    }

    fn kill_processes_in_namespace(&self) -> Result<(), HLError> {
        use nix::sys::signal::kill;
        use nix::sys::signal::Signal::{SIGTERM, SIGKILL};
        use std::thread::sleep;
        use std::time::Duration;

        let to_kill = try!(run_get_output_pids(
            &["ip", "netns", "pids", &self.name], self.env));
        if to_kill.len() == 0 { return Ok(()); }

        for pid in to_kill {
            if let Err(_) = kill(pid, SIGTERM) {
                // errors deliberately ignored
            }
        }

        sleep(Duration::from_secs(5));
        let to_kill = try!(run_get_output_pids(
            &["ip", "netns", "pids", &self.name], self.env));

        if to_kill.len() == 0 { return Ok(()); }
        for pid in to_kill {
            if let Err(_) = kill(pid, SIGKILL) {
                // errors deliberately ignored
            }
        }
        Ok(())
    }
}
impl<'a> Drop for NetNs<'a> {
    fn drop (&mut self) {
        if let Err(e) = self.kill_processes_in_namespace() {
            writeln!(io::stderr(), "{:?}", e).unwrap();
        }
        run_ignore_failure(&["ip", "netns", "exec", &self.name,
                             "ip", "link", "set", "dev", "lo", "down"],
                           self.env);
        run_ignore_failure(&["ip", "netns", "del", &self.name],
                           self.env);
    }
}

/// Create NNSP namespaces, named {PREFIX}_ns{N} where N is a number
/// from 0 to N-1.  Return their NetNs objects.
fn create_namespaces<'a>(prefix: &str, nnsp: u32, env: &'a ChildEnv)
                         -> Result<Vec<NetNs<'a>>, HLError> {
    let nnsp = nnsp as usize;
    let mut nsps: Vec<NetNs> = Vec::with_capacity(nnsp);
    for i in 0..nnsp {
        nsps.push(try!(NetNs::new(format!("{}_ns{}", prefix, i), env)));
        println!("{}", &nsps[i].name);
    }
    close_stdout();
    Ok(nsps)
}

/// Establish a safe set of environment variables for running child
/// processes.  TERM, TZ, LANG, and LC_* are passed down.  PATH is
/// forced to a known-good standard value.  All other environment
/// variables are discarded.  (The only subprogram run by this program
/// is "ip", which does not require HOME, USER, TMPDIR, etc.)
fn prepare_child_env() -> Vec<(String, String)> {
    let mut child_env: Vec<(String, String)> =
        env::vars().filter(|&(ref k, _)|
            k == "TERM" || k == "TZ" || k == "LANG" || k.starts_with("LC_")
        ).collect();

    child_env.push((String::from("PATH"),
                    String::from("/usr/local/bin:/usr/bin:/bin:\
                                  /usr/local/sbin:/usr/sbin:/sbin")));

    child_env.sort();
    child_env
}

/// Data parsed from the command line.
struct Args {
    prefix: String,
    n_namespaces: u32,
    dryrun: bool,
    verbose: bool
}

/// Parse the command line.
fn parse_cmdline() -> Args {
    use clap::{App,Arg,Error};
    use clap::ErrorKind::ValueValidation;

    let matches = App::new("tunnel-ns")
        .arg(Arg::with_name("prefix")
             .help("Prefix to use for the namespaces.  Must consist of \
                    ASCII letters, numbers, and underscores.")
             .index(1)
             .required(true)
             .empty_values(false))
        .arg(Arg::with_name("n_namespaces")
             .help("Number of namespaces to create (1-1024).")
             .index(2)
             .required(true)
             .empty_values(false))
        .arg(Arg::with_name("dryrun")
             .help("Do not perform any actions, just report \
                    what would have been done.")
             .short("n")
             .long("dryrun"))
        .arg(Arg::with_name("verbose")
             .help("Report all actions as they are executed.")
             .short("v")
             .long("verbose"))
        .get_matches();

    // This unwrap is safe because the value is marked 'required' above.
    let prefix = matches.value_of("prefix").unwrap();
    let nnsp   = value_t!(matches, "n_namespaces", u32)
        .unwrap_or_else(|e| e.exit());

    for c in prefix.chars() {
        if !(c.is_ascii() && (c.is_alphanumeric() || c == '_')) {
            Error::with_description(
                &format!("invalid prefix: {:?}", prefix),
                ValueValidation).exit();
        }
    }

    if nnsp < 1 || nnsp > 1024 {
        Error::with_description(
            &format!("n_namespaces must be from 1 to 1024, not {}", nnsp),
            ValueValidation).exit()
    }

    Args {
        prefix: String::from(prefix),
        n_namespaces: nnsp,
        verbose: (matches.is_present("verbose") ||
                  matches.is_present("dryrun")),
        dryrun: matches.is_present("dryrun")
    }
}


fn inner_main(args: Args) -> Result<(), HLError> {

    let (sigfd, child_mask) = try!(prepare_signals());

    let child_env = ChildEnv {
        env: prepare_child_env(),
        mask: child_mask,
        verbose: args.verbose,
        dryrun: args.dryrun
    };

    // _nsps exists solely so that the namespaces will be torn down
    // *after* the idle loop.
    let _nsps = try!(create_namespaces(&args.prefix,
                                       args.n_namespaces,
                                       &child_env));

    for ev in IdleLoop::new(sigfd) {
        match ev {
            Event::StdinClosed => {
                if args.verbose {
                    writeln!(io::stderr(), "# stdin closed, exiting").unwrap();
                }
                break;
            },
            Event::TermSignal(sig) => {
                if args.verbose {
                    writeln!(io::stderr(), "# {:?}, exiting", sig).unwrap();
                }
                break;
            },
            Event::ChildExit(pid) => {
                use nix::sys::wait::waitpid;
                let status = waitpid(pid, None).unwrap();
                writeln!(io::stderr(),
                         "# unexpected SIGCHLD(pid={}; status={:?})",
                         pid, status).unwrap();
            },
        }
    }
    Ok(())
}

fn main() {
    process::exit(match inner_main(parse_cmdline()) {
        Ok(_) => 0,
        Err(ref e) => {
            writeln!(io::stderr(), "{}", e).unwrap();
            1
        }
    });
}
