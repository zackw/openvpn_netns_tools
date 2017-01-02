//! The "idle loop" runs in between whatever setup and teardown
//! actions a program carries out.  It usually doesn't have much to
//! do, hence the name.

use std::io;
use std::mem;
use nix;

use std::io::{ErrorKind, Read, Write};
use std::os::unix::io::RawFd;
use nix::sys::signal::{Signal, SigSet, SIG_BLOCK};
use libc::{pid_t, c_int};

use err::*;

/// Internal: put a file descriptor into non-blocking mode.
fn make_nonblocking(fd: RawFd) -> Result<(), HLError> {
    use nix::fcntl::{fcntl, O_NONBLOCK};
    use nix::fcntl::FcntlArg::F_SETFL;

    fcntl(fd, F_SETFL(O_NONBLOCK))
        .map(|_| ())
        .map_err(|e| map_nix_err(e, format!("make_nonblocking({})", fd)))
}

/// Internal: Consume and discard data from standard input until
/// either EOF or EAGAIN.  Returns true for EOF, false for EAGAIN, or
/// an error.
fn consume_stdin() -> Result<bool, HLError> {
    let mut scratch: [u8; 4096] = unsafe { mem::uninitialized() };
    let mut stdin = io::stdin();
    loop {
        match stdin.read(&mut scratch) {
            Ok(0) => { return Ok(true); },
            Ok(_) => { continue; },
            Err(e) => {
                if e.kind() == ErrorKind::WouldBlock {
                    return Ok(false);
                } else {
                    return Err(map_io_err(e, String::from("stdin")));
                }
            }
        }
    }
}

// WNOWAIT isn't specified to work with waitpid.
// Neither nix nor libc exposes waitid.
// Feh.  Feh, I say.  Feh.
#[cfg(target_os = "macos")]
mod ffi {
    use libc::{c_int, siginfo_t};

    #[repr(C)]
    #[allow(dead_code)]
    #[allow(non_camel_case_types)]
    pub enum idtype_t { P_ALL, P_PID, P_PGID }
    #[allow(non_camel_case_types)]
    pub type id_t = u32;

    pub const WNOHANG : c_int = 0x00000001;
    pub const WEXITED : c_int = 0x00000004;
    pub const WNOWAIT : c_int = 0x00000020;

    extern {
        pub fn waitid(idtype: idtype_t, id: id_t,
                      infop: *mut siginfo_t, options: c_int) -> c_int;
    }
}

#[cfg(target_os = "linux")]
mod ffi {
    use libc::{c_int, siginfo_t};

    #[repr(C)]
    #[allow(dead_code)]
    #[allow(non_camel_case_types)]
    pub enum idtype_t { P_ALL, P_PID, P_PGID }
    #[allow(non_camel_case_types)]
    pub type id_t = u32;

    pub const WNOHANG : c_int = 1;
    pub const WEXITED : c_int = 4;
    pub const WNOWAIT : c_int = 0x01000000;

    extern {
        pub fn waitid(idtype: idtype_t, id: id_t,
                      infop: *mut siginfo_t, options: c_int) -> c_int;
    }
}

/// Internal: Poll for reapable child processes, if any.
/// Does not actually reap; caller is expected to do that.
fn poll_next_child() -> Option<pid_t> {
    use libc::siginfo_t;
    use nix::Errno;
    use self::ffi::*;

    let mut stat: siginfo_t = unsafe { mem::uninitialized() };
    let rv = unsafe { waitid(idtype_t::P_ALL,
                             0 as id_t,
                             &mut stat as *mut siginfo_t,
                             WEXITED|WNOHANG|WNOWAIT) };

    if rv == 0 {
        return if stat.si_pid == 0 { None } else { Some(stat.si_pid) };
    } else {
        let err = Errno::last();
        if err != Errno::ECHILD {
            writeln!(io::stderr(), "waitid: {}", err.desc()).unwrap();
        }
        return None;
    }
}

/// Return a signal set including all of the signals whose default
/// action is to terminate the process without a core dump.
fn sigset_normal_termination () -> SigSet {
    use nix::sys::signal::Signal::*;

    // It is easiest to define this signal set negatively.
    let mut ss = SigSet::all();

    // signals that cannot be caught
    ss.remove(SIGKILL);
    ss.remove(SIGSTOP);

    // signals that normally suspend the process
    ss.remove(SIGTSTP);
    ss.remove(SIGTTIN);
    ss.remove(SIGTTOU);

    // signals that are normally ignored
    // SIGCHLD is not in this list because we may need to respond to it
    ss.remove(SIGURG);
    ss.remove(SIGWINCH);

    // signals indicating a fatal CPU exception or user abort
    ss.remove(SIGABRT);
    ss.remove(SIGBUS);
    ss.remove(SIGFPE);
    ss.remove(SIGILL);
    ss.remove(SIGQUIT);
    ss.remove(SIGSEGV);
    ss.remove(SIGSYS);
    ss.remove(SIGTRAP);

    ss
}

/// Convert a Signal into a value that can be written to a pipe.
/// We know a priori that signal numbers are small, so we just take the
/// low 8 bits (checking for overflow).  Succeeds or crashes.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn serialize_signal (sig: Signal) -> u8 {
    let rv = sig as u32;
    assert!(rv < 256);
    rv as u8
}

/// The inverse operation.  Succeeds or crashes.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn deserialize_signal (sig: u8) -> Signal {
    Signal::from_c_int(sig as c_int).unwrap()
}

/// This function implements the "self-pipe trick" for plumbing signals
/// into a select() operation.  It is used on systems that do not support
/// signalfd().
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn start_signal_worker (sigs: SigSet) -> Result<RawFd, HLError> {
    use nix::unistd::{pipe, write};
    use std::thread::spawn;

    let (rd, wr) = try!(pipe()
                        .map_err(|e| map_nix_err(e, String::from("pipe"))));

    try!(make_nonblocking(rd));

    // There is no good way to tell this thread to exit (because it'll
    // be blocked in sigwait() all the time), so we just drop the
    // handle.  Failures in the thread should be impossible.
    spawn(move || {
        loop {
            let sig = [serialize_signal(sigs.wait().unwrap())];
            write(wr, &sig).unwrap();
        }
    });

    Ok(rd)
}

/// This function reads from the self-pipe and regenerates Signal objects.
/// When the pipe is drained it returns None.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn next_signal(fd: RawFd) -> Option<Signal> {
    use nix::unistd::read;
    use nix::Errno::EAGAIN;

    let mut buf : [u8;1] = unsafe { mem::uninitialized() };
    match read(fd, &mut buf) {
        Err(nix::Error::Sys(EAGAIN)) => None,
        Ok(0) => None,
        Ok(1) => Some(deserialize_signal(buf[0])),

        Err(e) => panic!("next_signal: {:?}", e),
        Ok(n) => panic!("next_signal: read too many bytes ({})", n)
    }
}

/// Prepare signal handling.  This records the original signal mask
/// so it can be restored in child processes, establishes a signal mask
/// that blocks all the signals we want to pick up via the worker thread
/// or signalfd(), and starts the thread / signalfd going.
/// Must be called before creating any threads, so that the
/// signal mask is established globally.
pub fn prepare_signals() -> Result<(RawFd, SigSet), HLError> {
    let parent_mask = sigset_normal_termination();
    let child_mask = try!(
        parent_mask.thread_swap_mask(SIG_BLOCK)
            .map_err(|e| map_nix_err(e, String::from("sigprocmask"))));

    let sigpipe = try!(start_signal_worker(parent_mask));

    Ok((sigpipe, child_mask))
}

/// The std::io API currently provides no way to close stdout, so this
/// function does it with primitives.  To avoid problems due to the
/// library-level file handle remaining open, after closing fd 1 we
/// duplicate fd 2 down to 1 (so anything written to stdout after this
/// function is called will wind up on stderr).  This function either
/// succeeds, or crashes the program.
pub fn close_stdout() {
    use nix::unistd::{close, dup2};

    // Note: fd 1 will have been closed _even if_ the close returns an
    // error code.  Just report any error and move on.
    if let Err(e) = close(1) {
        writeln!(io::stderr(), "stdout: {}", e).unwrap();
    }

    // If this step fails (which should never happen), low-level state
    // is inconsistent and it's not safe to continue, so we crash.
    dup2(2, 1).expect("Failed to cover stdout with stderr");
}

/// An "event" is anything that the main program might need to take
/// notice of.  Currently these are:
///  - stdin has been closed
///  - the program received a signal that should trigger a graceful exit
///  - an asynchronous child process has exited
pub enum Event {
    StdinClosed,
    TermSignal(Signal),
    ChildExit(pid_t),
}

// An IdleLoop is a generator of Events.
pub struct IdleLoop {
    signal_pipe:  RawFd,
    stdin_closed: bool,
    stdin_pending: bool,
    signal_pending: bool,
    children_pending: bool
}
impl IdleLoop {
    pub fn new (signal_pipe: RawFd) -> IdleLoop {
        IdleLoop {
            signal_pipe: signal_pipe,
            stdin_closed: false,
            stdin_pending: false,
            signal_pending: false,
            children_pending: false
        }
    }
    fn poll (&mut self) {
        use nix::poll::{poll, PollFd, POLLIN, EventFlags};

        if self.stdin_closed {
            let mut pfds = [PollFd::new(self.signal_pipe, POLLIN,
                                        EventFlags::empty())];

            poll(&mut pfds, -1).unwrap();
            if !pfds[0].revents().unwrap().is_empty() {
                self.signal_pending = true;
            }

        } else {
            let mut pfds = [PollFd::new(self.signal_pipe, POLLIN,
                                        EventFlags::empty()),
                            PollFd::new(0 /* stdin */, POLLIN,
                                        EventFlags::empty())];
            poll(&mut pfds, -1).unwrap();
            if !pfds[0].revents().unwrap().is_empty() {
                self.signal_pending = true;
            }
            if !pfds[1].revents().unwrap().is_empty() {
                self.stdin_pending = true;
            }
        }
    }

    pub fn next_event (&mut self) -> Event {
        loop {
            if !self.stdin_pending
                && !self.signal_pending
                && !self.children_pending {
                    self.poll();
                }
            if self.stdin_pending {
                self.stdin_pending = false;
                match consume_stdin() {
                    Ok(false) => (),
                    Ok(true) => {
                        self.stdin_closed = true;
                        return Event::StdinClosed;
                    }
                    Err(e) => {
                        writeln!(io::stderr(), "stdin: {}", e).unwrap();
                        // Assume stdin is no good anymore.
                        self.stdin_closed = true;
                        return Event::StdinClosed;
                    }
                }
            }
            if self.signal_pending {
                match next_signal(self.signal_pipe) {
                    None => {
                        self.signal_pending = false;
                    },
                    Some(Signal::SIGCHLD) => {
                        self.children_pending = true;
                    },
                    Some(sig) => {
                        return Event::TermSignal(sig);
                    }
                }
            }
            if self.children_pending {
                match poll_next_child() {
                    Some(pid) => {
                        return Event::ChildExit(pid);
                    },
                    None => {
                        self.children_pending = false;
                    }
                }
            }
        }
    }
}
impl Iterator for IdleLoop {
    type Item = Event;
    fn next(&mut self) -> Option<Self::Item> {
        Some(self.next_event())
    }
}
