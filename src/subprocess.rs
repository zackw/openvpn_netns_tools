/// Subprocess management.

use std::io;
use std::num;
use std::str;

use std::io::Write;
use std::process::{Child,Command,Stdio,ExitStatus};
use nix::sys::signal::SigSet;
//use nix::sys::signal::SIG_SETMASK;
//use std::os::unix::process::CommandExt;
use libc::pid_t;

use err::*;

#[allow(dead_code)] // until we turn sigmasks back on
pub struct ChildEnv {
    pub env:  Vec<(String, String)>,
    pub mask: SigSet,
    pub verbose: bool,
    pub dryrun: bool,
}

fn internal_spawn(argv: &[&str], env: &ChildEnv, stdout: Stdio)
                  -> io::Result<Child> {

    if env.verbose {
        writeln!(io::stderr(), "{}", argv.join(" ")).unwrap();
    }

    let exe = if env.dryrun { "true" } else { argv[0] };

    let mut cmd = Command::new(exe);
    cmd.stdin(Stdio::null());
    cmd.stdout(stdout);
    cmd.args(&argv[1..]);
    cmd.env_clear();

    for &(ref k, ref v) in env.env.iter() {
        cmd.env(k, v);
    }
/*
    cmd.before_exec(|| {
        pthread_sigmask(SIG_SETMASK, Some(env.mask), None)
    });
*/
    cmd.spawn()
}

fn check_child_status(argv: &[&str], status: &ExitStatus)
                      -> Result<(), HLError> {
    if status.success() {
        Ok(())
    } else {
        Err(map_unsuc_child(status, argv))
    }
}

pub fn spawn(argv: &[&str], env: &ChildEnv) -> Result<Child, HLError> {
    internal_spawn(argv, env, Stdio::inherit())
        .map_err(|e| map_io_err(e, format!("spawn {}", argv[0])))
}

pub fn run(argv: &[&str], env: &ChildEnv) -> Result<(), HLError> {

    let mut child = try!(spawn(argv, env));
    let status = try!(child.wait()
                      .map_err(|e| map_io_err(e, format!("wait for {}",
                                                         argv[0]))));

    check_child_status(argv, &status)
}

pub fn run_ignore_failure(argv: &[&str], env: &ChildEnv) {
    match run(argv, env) {
        Ok(_) => (),
        Err(e) => {
            writeln!(io::stderr(), "{}", e).unwrap();
        }
    }
}

pub fn run_get_output(argv: &[&str], env: &ChildEnv)
                      -> Result<Vec<u8>, HLError> {
    let child = try!(internal_spawn(argv, env, Stdio::piped())
                     .map_err(|e| map_io_err(e, format!("spawn {}",
                                                        argv[0]))));
    let output = try!(child.wait_with_output()
                      .map_err(|e| map_io_err(e, format!("reading from {}",
                                                         argv[0]))));

    try!(check_child_status(argv, &output.status));
    Ok(output.stdout)
}

pub fn run_get_output_pids(argv: &[&str], env: &ChildEnv)
                           -> Result<Vec<pid_t>, HLError> {

    let raw_output = try!(run_get_output(argv, env));
    let output = try!(str::from_utf8(&raw_output)
                      .map_err(|e| map_utf8_err(e, format!("{:?}",
                                                           raw_output))));

    output
        .split_whitespace().map(|s| s.parse::<pid_t>())
        .collect::<Result<Vec<pid_t>, num::ParseIntError>>()
        .map_err(|e| map_pi_err(e, String::from("expected process id")))
}
