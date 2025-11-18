# llrt_dgram

LLRT implementation of Node.js `dgram` module for UDP datagram sockets.

## Features

- UDP socket support (IPv4 and IPv6)
- Send and receive datagrams
- Event-driven API compatible with Node.js
- Async/await support

## Supported APIs

- `dgram.createSocket(type[, callback])`
- `socket.send(msg, port, address[, callback])`
- `socket.bind([port][, address][, callback])`
- `socket.close([callback])`
- `socket.address()`
- `socket.unref()`
- `socket.ref()`

## Events

- `'message'` - Emitted when a new datagram is available
- `'listening'` - Emitted when socket begins listening for datagrams
- `'close'` - Emitted after socket is closed
- `'error'` - Emitted when an error occurs
