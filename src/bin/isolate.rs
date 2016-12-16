/* Wrapper for invoking programs in a weakly isolated environment.
 *
 * Copyright © 2014 Zack Weinberg
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 * http://www.apache.org/licenses/LICENSE-2.0
 * There is NO WARRANTY.
 *
 *    isolate [VAR=val...] program [args...]
 *
 * runs 'program' with arguments 'args' under its own user and group
 * ID, in a just-created, (almost) empty home directory, in its own
 * background process group.  stdin, stdout, and stderr are inherited
 * from the parent, and all other file descriptors are closed.
 * Resource limits are applied (see below).  When 'program' exits,
 * everything else in its process group is killed, and its home
 * directory is erased.
 *
 * HOME, USER, PWD, LOGNAME, and SHELL are set appropriately; TMPDIR
 * is set to $HOME/.tmp, which is created along with $HOME; PATH, TZ,
 * TERM, LANG, and LC_* are preserved; all other environment variables
 * are cleared.  'VAR=val' arguments to isolate, prior to 'program',
 * set additional environment variables for 'program', a la env(3).
 * The first argument that does not match /^[A-Za-z_][A-Za-z0-9_]*=/
 * is taken as 'program', and all subsequent arguments are passed to
 * 'program' verbatim.
 *
 * VARs with names starting ISOL_*, on the command line, may be used
 * to adjust the behavior of this program, and will not be passed
 * down.  These are *not* honored if set in this program's own
 * environment variable block.  Unrecognized ISOL_* variables are a
 * fatal error.
 *
 * This program is to be installed setuid root.
 *
 * The directory ISOL_HOME (default /home/isolated) must exist, be
 * owned by root, and not be used for any other purpose.
 *
 * The userid range ISOL_LOW_UID (default 2000) through ISOL_HIGH_UID
 * (default 2999), inclusive, must not conflict with any existing user
 * or group ID.  If you put this uid range in /etc/passwd and
 * /etc/group, the username, group membership and shell specified
 * there (but *not* the homedir) will be honored; otherwise, the
 * process will be given a primary GID with the same numeric value as
 * its UID, no supplementary groups, USER and LOGNAME will be set to
 * "iso-NNNN" where NNNN is the decimal UID, and SHELL will be set to
 * "/bin/sh".
 *
 * If ISOL_NETNS is set to any value, this program reexecs itself
 * under "ip netns exec $value" before doing anything else, thus
 * arranging for the subsidiary program to run in a non-default
 * network namespace (which must already have been established).
 *
 * There are twelve parameters of the form "ISOL_RL_<limit>" -- see
 * below for a list -- which can be used to set resource limits on the
 * isolated program.  Most, but not all, of the <limit>s correspond to
 * RLIM_<limit> constants from sys/resource.h and are enforced via
 * setrlimit(2).  The exceptions are ISOL_RL_WALL, which places a
 * limit on *wall-clock* execution time (enforced by watchdog timer in
 * the parent process) and ISOL_RL_MEM, which sets all three of
 * RLIMIT_AS, RLIMIT_DATA, and RLIMIT_RSS; those three cannot be set
 * individually.
 *
 * This program is not intended as a replacement for full-fledged
 * containers!  The subsidiary program can still access the entire
 * filesystem and all other shared resources.  It can spawn children
 * that remove themselves from its process group, and thus escape
 * termination when their parent exits.  There is no attempt to set
 * extended credentials of any kind, or apply PAM session settings, or
 * anything like that.  But on the up side, you don't have to
 * construct a chroot environment.
 *
 * This program has only been tested on Linux.  C99 and POSIX.1-2001
 * features are used throughout.  It also requires static_assert, from
 * C11; dirfd, lchown, and strdup, from POSIX.1-2008; and execvpe,
 * initgroups, and vasprintf, from the shared BSD/GNU extension set.
 *
 * It should not be difficult to port this program to any modern *BSD,
 * but it may well be impractical to port it to anything older.
 */

fn main() {
    unimplemented!()
}
