// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
import { createServer as createTcpServer } from 'node:net'

/**
 * Minimal Node.js-like HTTP server implementation using net.
 * @param {(req: IncomingMessage, res: ServerResponse) => void} listener
 * @returns {import('net').Server}
 *
 * @example
 * import { createServer } from 'node:http';
 *
 * const server = createServer((req, res) => {
 *   console.log(`${req.method} ${req.url}`);
 *   res.writeHead(200, { 'Content-Type': 'text/plain' });
 *   res.end('Hello Compatible Server');
 * });
 *
 * server.listen(3000, () => {
 *   console.log('Server running at http://localhost:3000/');
 * });
 *
 * // Output when you curl:
 * // $ curl http://localhost:3000/hello
 * // Hello Compatible Server
 */
export function createServer(listener) {
  return createTcpServer((socket) => {
    socket.on('error', (error) => {
      console.error('Socket error:', error)
      socket.end()
    })

    socket.on('data', async (data) => {
      try {
        const requestString = data.toString()
        const [headerPart, bodyPart = ''] = requestString.split('\r\n\r\n')
        const lines = headerPart.split('\r\n')
        const [method, path, protocol] = lines[0].split(' ')

        const headers = {}
        for (let i = 1; i < lines.length; i++) {
          const line = lines[i]
          if (line) {
            const [key, ...valueParts] = line.split(': ')
            headers[key.toLowerCase()] = valueParts.join(': ')
          }
        }

        // --- Simplified IncomingMessage object ---
        const req = {
          method,
          url: path,
          headers,
          httpVersion: protocol.replace('HTTP/', ''),
          socket,
          body: bodyPart,
        }

        // --- Simplified ServerResponse object ---
        let headersSent = false
        const resHeaders = {}
        const res = {
          statusCode: 200,
          statusMessage: 'OK',
          setHeader(name, value) {
            resHeaders[name] = value
          },
          getHeader(name) {
            return resHeaders[name]
          },
          writeHead(statusCode, statusMessageOrHeaders, maybeHeaders) {
            if (headersSent) return
            if (typeof statusMessageOrHeaders === 'string') {
              this.statusCode = statusCode
              this.statusMessage = statusMessageOrHeaders
              Object.assign(resHeaders, maybeHeaders || {})
            } else {
              this.statusCode = statusCode
              this.statusMessage = 'OK'
              Object.assign(resHeaders, statusMessageOrHeaders || {})
            }
            const headerLines = Object.entries(resHeaders)
              .map(([k, v]) => `${k}: ${v}`)
              .join('\r\n')
            socket.write(`HTTP/1.1 ${this.statusCode} ${this.statusMessage}\r\n${headerLines}\r\n\r\n`)
            headersSent = true
          },
          write(chunk) {
            if (!headersSent) {
              this.writeHead(this.statusCode)
            }
            socket.write(chunk)
          },
          end(chunk) {
            if (chunk) this.write(chunk)
            socket.end()
          },
        }

        // Call the user-defined listener
        listener(req, res)
      } catch (error) {
        console.error('Error handling request:', error)
        socket.write('HTTP/1.1 500 Internal Server Error\r\n\r\n')
        socket.end()
      }
    })
  })
}

export default { createServer }
