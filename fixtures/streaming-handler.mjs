import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";

export const handler = awslambda.streamifyResponse(
  async (event, responseStream) => {
    responseStream.setContentType("text/plain");
    responseStream.write("Hello, ");
    // Long enough for the test to see that the first chunk was sent before the handler finished
    await new Promise((resolve) => setTimeout(resolve, 300));
    responseStream.write(event.name);
    responseStream.end("!");
  }
);

// The pattern @middy/core uses
export const pipelineHandler = awslambda.streamifyResponse(
  async (_event, responseStream) => {
    await pipeline(
      Readable.from([
        `highWaterMark=${responseStream.writableHighWaterMark};`,
        "a",
        "b",
        "c",
      ]),
      responseStream
    );
  },
  { highWaterMark: 4 }
);

// The invocation lasts until the stream ends, not until the handler returns
export const endsAfterReturn = awslambda.streamifyResponse(
  async (_event, responseStream) => {
    setTimeout(() => responseStream.end("ended later"), 50);
  }
);

export const lateContentType = awslambda.streamifyResponse(
  async (_event, responseStream) => {
    // An empty write opens the response, as hono/aws-lambda does for empty bodies
    responseStream.write("");
    try {
      responseStream.setContentType("text/html");
    } catch (error) {
      responseStream.end(error.message);
      return;
    }
    responseStream.end("setContentType did not throw");
  }
);

export const throwsAfterWrite = awslambda.streamifyResponse(
  async (_event, responseStream) => {
    responseStream.write("partial");
    throw new Error("kaboom");
  }
);

export const throwsBeforeWrite = awslambda.streamifyResponse(async () => {
  throw new Error("kaboom");
});

export const notAsync = awslambda.streamifyResponse(
  (_event, responseStream) => {
    responseStream.write("partial");
  }
);

// As in Node.js, chunks other than strings and Uint8Arrays are sent as JSON
export const writesJson = awslambda.streamifyResponse(
  async (_event, responseStream) => {
    responseStream.write({ a: 1 });
    responseStream.write(null);
    responseStream.write([1, 2]);
    responseStream.write(Buffer.from("bytes"));
    responseStream.end(new Uint8Array([33]));
  }
);

// Writes 1 MiB chunks until a 'drain' waits at least 500 ms for the reader,
// then reports what happened
export const waitsForDrain = awslambda.streamifyResponse(
  async (_event, responseStream) => {
    const chunk = "x".repeat(1024 * 1024);
    let writes = 0;
    let falses = 0;
    let longestWait = 0;
    while (longestWait < 500 && writes < 64) {
      writes++;
      if (!responseStream.write(chunk)) {
        falses++;
        const start = Date.now();
        await new Promise((resolve) => responseStream.once("drain", resolve));
        longestWait = Math.max(longestWait, Date.now() - start);
      }
    }
    responseStream.end(
      `;writes=${writes};falses=${falses};longestWait=${longestWait}`
    );
  }
);

// Writes until the runtime destroys the stream, as it does when the Runtime
// API refuses the response
export const writesUntilDestroyed = awslambda.streamifyResponse(
  async (_event, responseStream) => {
    const chunk = "x".repeat(1024);
    for (let i = 0; i < 64 * 1024; i++) {
      if (responseStream.destroyed) {
        return;
      }
      if (!responseStream.write(chunk)) {
        await new Promise((resolve) => {
          const done = () => {
            responseStream.off("drain", done);
            responseStream.off("close", done);
            resolve();
          };
          responseStream.on("drain", done);
          responseStream.on("close", done);
        });
      }
    }
    responseStream.end("never destroyed");
  }
);

// The stream's `end` fails while the runtime reports the handler's error
export const endThrows = awslambda.streamifyResponse(
  async (_event, responseStream) => {
    responseStream.write("partial");
    responseStream.end = () => {
      throw new Error("end failed");
    };
    throw new Error("kaboom");
  }
);

export const pipelineSourceFails = awslambda.streamifyResponse(
  async (_event, responseStream) => {
    const source = new Readable({
      read() {
        this.destroy(new Error("source failed"));
      },
    });
    await pipeline(source, responseStream);
  }
);
