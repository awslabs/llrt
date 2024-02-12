// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
import { EventEmitter as EE } from "events";
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

function inherits(ctor: any, superCtor: any) {
  if (superCtor) {
    ctor.super_ = superCtor;
    ctor.prototype = Object.create(superCtor.prototype, {
      constructor: {
        value: ctor,
        enumerable: false,
        writable: true,
        configurable: true,
      },
    });
  }
}
inherits(Stream, EE);
Stream.Readable = Readable;
Stream.Writable = Writable;
Stream.Duplex = Duplex;
Stream.Transform = Transform;
Stream.PassThrough = PassThrough;
Stream.finished = finished;
Stream.pipeline = pipeline;
Stream.Stream = Stream;

// old-style streams.  Note that the pipe method (the only relevant
// part of this class) is overridden in the Readable class.

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
