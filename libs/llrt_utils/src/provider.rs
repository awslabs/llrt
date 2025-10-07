// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
#[derive(PartialEq)]
pub enum ProviderType {
    None,
    Resource(String), // Custom asynchronous resource
    // Userland provider types
    Immediate,   // [Immediate] Processing by setImmediate()
    Interval,    // [Interval] Timer by setInterval()
    MessagePort, // [MessagePort] Port for worker_threads
    Microtask,   // [Microtask] Processing by queueMicrotask()
    TickObject,  // [TickObject] Processing by process.nextTick()
    Timeout,     // [Timeout] Timer by setTimeout()
    // Internal provider types
    FsReqCallback,      // [FSREQCALLBACK] Callback for file system operations
    GetAddrInfoReqWrap, // [GETADDRINFOREQWRAP] When resolving DNS (dns.lookup(), etc.)
    GetNameInfoReqWrap, // [GETNAMEINFOREQWRAP] DNS reverse lookup
    PipeWrap,           // [PIPEWRAP] Pipe connection
    StatWatcher,        // [STATWACHER] File monitoring such as fs.watch()
    TcpWrap,            // [TCPWRAP] TCP socket wrap (net.Socket, etc.)
    TimerWrap,          // [TIMERWRAP] Internal timer wrap (low level)
    TlsWrap,            // [TLSWRAP] TLS socket (HTTPS, etc.)
    UdpWrap,            // [UDPWRAP] UDP socket wrap (dgram module)
}
