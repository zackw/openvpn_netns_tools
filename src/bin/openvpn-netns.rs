/* Establish network namespaces that use OpenVPN for all communication.
 *
 * Copyright © 2014 Zack Weinberg
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 * http://www.apache.org/licenses/LICENSE-2.0
 * There is NO WARRANTY.
 *
 *     openvpn-netns namespace config-file [args...]
 *
 * brings up an OpenVPN tunnel which network namespace NAMESPACE will
 * use for communication.  NAMESPACE must already exist.  (The program
 * 'tunnel-ns' sets up namespaces appropriately.)  CONFIG-FILE is an
 * OpenVPN configuration file, and any ARGS will be appended to the
 * OpenVPN command line.
 *
 * This program expects to be run with both stdin and stdout connected
 * to pipes.  When it detects that the namespace is ready for use, it
 * will write the string "READY\n" to its stdout and then close it.
 * It expects that nothing will be written to its stdin (anything that
 * *is* written will be read and discarded), but when stdin is closed,
 * it will terminate the OpenVPN client, tear down the network
 * namespace (and terminate all processes still in there), and exit.
 *
 * Error messages, and any output from the OpenVPN client, will be
 * written to stderr.  One may wish to include "--verb 0" in ARGS to
 * make the client less chatty.
 *
 * This program must be installed setuid root.
 *
 * This program makes extensive use of Linux-specific network stack
 * features.  A port to a different OS might well entail a complete
 * rewrite.  Apart from that, C99 and POSIX.1-2001 features are used
 * throughout.  It also requires dirfd, strdup, and strsignal, from
 * POSIX.1-2008; execvpe, pipe2, and vasprintf, from the shared
 * BSD/GNU extension set; and the currently Linux-specific signalfd
 * and getauxval.
 */

fn main() {
    unimplemented!()
}
