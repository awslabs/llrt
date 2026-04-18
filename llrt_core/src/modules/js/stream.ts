// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
import { EventEmitter as EE } from "node:events";
// @ts-ignore
import { Readable } from "readable-stream/lib/_stream_readable.js";
// @ts-ignore
import { Writable } from "readable-stream/lib/_stream_writable.js";
// @ts-ignore
import { Duplex } from "readable-stream/lib/_stream_duplex.js";
// @ts-ignore
import { Transform } from "readable-stream/lib/_stream_transform.js";
// @ts-ignore
import { PassThrough } from "readable-stream/lib/_stream_passthrough.js";
// @ts-ignore
import { finished } from "readable-stream/lib/internal/streams/end-of-stream.js";
// @ts-ignore
import { pipeline } from "readable-stream/lib/internal/streams/pipeline.js";

import { inherits } from "node:util";

inherits(Stream, EE);
Stream.Readable = Readable;
Stream.Writable = Writable;
Stream.Duplex = Duplex;
Stream.Transform = Transform;
Stream.PassThrough = PassThrough;
Stream.finished = finished;
Stream.pipeline = pipeline;
Stream.Stream = Stream;

// Web stream adapters — bridge bundled Node.js streams to native web streams
const enc = new TextEncoder();

Readable.toWeb = function (readable: any) {
  const iterator = readable[Symbol.asyncIterator]();
  return new ReadableStream({
    async pull(controller: any) {
      const { value, done } = await iterator.next();
      if (done) {
        controller.close();
      } else {
        controller.enqueue(
          typeof value === "string" ? enc.encode(value) : value
        );
      }
    },
    cancel(reason: any) {
      readable.destroy(reason);
    },
  });
};

Readable.fromWeb = function (readableStream: any, options?: any) {
  const reader = readableStream.getReader();
  return new Readable({
    ...options,
    async read() {
      const { value, done } = await reader.read();
      this.push(done ? null : value);
    },
  });
};

Writable.toWeb = function (writable: any) {
  return new WritableStream({
    write(chunk: any) {
      return new Promise<void>((resolve) => {
        if (!writable.write(chunk)) {
          writable.once("drain", resolve);
        } else {
          resolve();
        }
      });
    },
    close() {
      return new Promise<void>((resolve) => {
        writable.end(resolve);
      });
    },
    abort(reason: any) {
      writable.destroy(reason);
    },
  });
};

Writable.fromWeb = function (writableStream: any, options?: any) {
  const writer = writableStream.getWriter();
  return new Writable({
    ...options,
    async write(chunk: any, _encoding: any, callback: any) {
      try {
        await writer.write(chunk);
        callback();
      } catch (e) {
        callback(e);
      }
    },
    final(callback: any) {
      writer.close().then(callback, callback);
    },
  });
};

Duplex.toWeb = function (duplex: any) {
  return {
    readable: Readable.toWeb(duplex),
    writable: Writable.toWeb(duplex),
  };
};

Duplex.fromWeb = function (pair: any, options?: any) {
  const reader = pair.readable.getReader();
  const writer = pair.writable.getWriter();
  return new Duplex({
    ...options,
    async read() {
      const { value, done } = await reader.read();
      this.push(done ? null : value);
    },
    async write(chunk: any, _encoding: any, callback: any) {
      try {
        await writer.write(chunk);
        callback();
      } catch (e) {
        callback(e);
      }
    },
    final(callback: any) {
      writer.close().then(callback, callback);
    },
  });
};

function Stream(this: any) {
  EE.call(this);
}

Stream.prototype.pipe = function (dest: any, options: any) {
  var source = this;

  function ondata(chunk: any) {
    if (dest.writable) {
      if (false === dest.write(chunk) && source.pause) {
        source.pause();
      }
    }
  }

  source.on("data", ondata);

  function ondrain() {
    if (source.readable && source.resume) {
      source.resume();
    }
  }

  dest.on("drain", ondrain);

  // If the 'end' option is not supplied, dest.end() will be called when
  // source gets the 'end' or 'close' events.  Only dest.end() once.
  if (!dest._isStdio && (!options || options.end !== false)) {
    source.on("end", onend);
    source.on("close", onclose);
  }

  var didOnEnd = false;
  function onend() {
    if (didOnEnd) return;
    didOnEnd = true;

    dest.end();
  }

  function onclose() {
    if (didOnEnd) return;
    didOnEnd = true;

    if (typeof dest.destroy === "function") dest.destroy();
  }

  // don't leave dangling pipes when there are errors.
  function onerror(er: any) {
    cleanup();
    if (source.listenerCount("error") === 0) {
      throw er; // Unhandled stream error in pipe.
    }
  }

  source.on("error", onerror);
  dest.on("error", onerror);

  // remove all the event listeners that were added.
  function cleanup() {
    source.removeListener("data", ondata);
    dest.removeListener("drain", ondrain);

    source.removeListener("end", onend);
    source.removeListener("close", onclose);

    source.removeListener("error", onerror);
    dest.removeListener("error", onerror);

    source.removeListener("end", cleanup);
    source.removeListener("close", cleanup);

    dest.removeListener("close", cleanup);
  }

  source.on("end", cleanup);
  source.on("close", cleanup);

  dest.on("close", cleanup);

  dest.emit("pipe", source);

  // Allow for unix-like usage: A.pipe(B).pipe(C)
  return dest;
};

export default Stream;
export {
  Readable,
  Writable,
  Duplex,
  Transform,
  PassThrough,
  finished,
  pipeline,
  Stream,
};
