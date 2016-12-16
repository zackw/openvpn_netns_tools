/* Establish network namespaces that will use tunnel devices as their
 * default routes.
 *
 * Copyright © 2015 Zack Weinberg
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 * http://www.apache.org/licenses/LICENSE-2.0
 * There is NO WARRANTY.
 *
 *     tunnel-ns PREFIX N
 *
 * creates N network namespaces, imaginatively named PREFIX_ns0,
 * PREFIX_ns1, ... The loopback device in each namespace is brought
 * up, with the usual address.  /etc/netns directories for each
 * namespace are created.  No other setup is performed.  (The tunnel
 * interfaces are expected to be created on the fly by a program like
 * 'openvpn-netns', which see.  This is because (AFAICT) if you create
 * a persistent tunnel ahead of time, and put its interface side into
 * a namespace, it then becomes impossible for anything to reattach
 * to the device side.)
 *
 * This program expects to be run with both stdin and stdout connected
 * to pipes.  As it creates each namespace, it writes one line to its
 * stdout:
 *
 *   PREFIX_nsX <newline>
 *
 * After all namespaces have been created, stdout is closed.
 *
 * Anything written to stdin is read and discarded.  When stdin is
 * *closed*, however, all of the network namespaces are torn down
 * (killing any processes still in there, if necessary) and the
 * program exits.  This also happens on receipt of any catchable
 * signal whose default action is to terminate the process without
 * a core dump (e.g. SIGTERM, SIGHUP).
 *
 * Errors, if any, will be written to stderr.
 *
 * This program must be installed setuid root.
 *
 * This program makes extensive use of Linux-specific network stack
 * features.  A port to a different OS might well entail a complete
 * rewrite.  Apart from that, C99 and POSIX.1-2001 features are used
 * throughout.  It also requires dirfd, strdup, and strsignal, from
 * POSIX.1-2008; execvpe, pipe2, and vasprintf, from the shared
 * BSD/GNU extension set; and the currently Linux-specific signalfd.
 */

fn main() {
    unimplemented!()
}
