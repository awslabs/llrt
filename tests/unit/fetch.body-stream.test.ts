describe("Fetch Body Stream", () => {
  describe("Response.body", () => {
    it("should return null for empty response", () => {
      const response = new Response();
      expect(response.body).toEqual(null);
    });

    it("should return null for null body", () => {
      const response = new Response(null);
      expect(response.body).toEqual(null);
    });

    it("should return ReadableStream for string body", () => {
      const response = new Response("Hello, World!");
      expect(response.body).toBeInstanceOf(ReadableStream);
    });

    it("should return ReadableStream for Uint8Array body", () => {
      const data = new Uint8Array([1, 2, 3, 4, 5]);
      const response = new Response(data);
      expect(response.body).toBeInstanceOf(ReadableStream);
    });

    it("should return ReadableStream for Blob body", () => {
      const blob = new Blob(["test content"], { type: "text/plain" });
      const response = new Response(blob);
      expect(response.body).toBeInstanceOf(ReadableStream);
    });

    it("should be able to read body stream with getReader", async () => {
      const text = "Hello, Stream!";
      const response = new Response(text);
      const reader = response.body!.getReader();

      const chunks: Uint8Array[] = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }

      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(text);
    });

    it("should set bodyUsed to true after consuming body", async () => {
      const response = new Response("test");
      expect(response.bodyUsed).toBeFalsy();
      // Accessing body doesn't set bodyUsed - consuming it does
      const body = response.body;
      expect(response.bodyUsed).toBeFalsy();
      // Reading from the stream marks the body as used
      await response.text();
      expect(response.bodyUsed).toBeTruthy();
    });

    it("should return same stream on multiple body accesses", () => {
      const response = new Response("test");
      const body1 = response.body;
      const body2 = response.body;
      // Per spec: body getter returns the same stream
      expect(body1).toBeInstanceOf(ReadableStream);
      expect(body2).toBeInstanceOf(ReadableStream);
      // They should be the same stream object
      expect(body1).toBe(body2);
    });

    it("should return null after body is consumed via text()", async () => {
      const response = new Response("test");
      await response.text();
      // After consuming with text(), body should be null
      expect(response.body).toEqual(null);
    });
  });

  describe("Request.body", () => {
    it("should return null for request without body", () => {
      const request = new Request("https://example.com");
      expect(request.body).toEqual(null);
    });

    it("should return ReadableStream for POST request with string body", () => {
      const request = new Request("https://example.com", {
        method: "POST",
        body: "Hello, World!",
      });
      expect(request.body).toBeInstanceOf(ReadableStream);
    });

    it("should return ReadableStream for POST request with Uint8Array body", () => {
      const data = new Uint8Array([1, 2, 3, 4, 5]);
      const request = new Request("https://example.com", {
        method: "POST",
        body: data,
      });
      expect(request.body).toBeInstanceOf(ReadableStream);
    });

    it("should be able to read request body stream", async () => {
      const text = "Request body content";
      const request = new Request("https://example.com", {
        method: "POST",
        body: text,
      });

      const reader = request.body!.getReader();
      const chunks: Uint8Array[] = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }

      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(text);
    });

    it("should set bodyUsed to true after accessing body", () => {
      const request = new Request("https://example.com", {
        method: "POST",
        body: "test",
      });
      expect(request.bodyUsed).toBeFalsy();
      const _body = request.body;
      expect(request.bodyUsed).toBeTruthy();
    });
  });

  describe("Response body stream from fetch", () => {
    it("should return ReadableStream from fetch response", async () => {
      // Using a data URL to avoid network dependency
      const response = await fetch("data:text/plain,Hello%20World");
      expect(response.body).toBeInstanceOf(ReadableStream);
    });

    it("should be able to read fetch response body stream", async () => {
      const expectedText = "Hello World";
      const response = await fetch("data:text/plain,Hello%20World");

      const reader = response.body!.getReader();
      const chunks: Uint8Array[] = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }

      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(expectedText);
    });
  });

  describe("ReadableStream as Request body", () => {
    it("should accept ReadableStream as Request body", () => {
      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode("streamed body"));
          controller.close();
        },
      });

      const request = new Request("https://example.com", {
        method: "POST",
        body: stream,
      });

      // body should return the stream (or a new stream wrapping it)
      expect(request.body).toBeInstanceOf(ReadableStream);
    });

    it("should be able to read Request body when created with ReadableStream", async () => {
      const text = "Hello from stream!";
      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode(text));
          controller.close();
        },
      });

      const request = new Request("https://example.com", {
        method: "POST",
        body: stream,
      });

      // Use text() to read the body
      const result = await request.text();
      expect(result).toEqual(text);
    });
  });

  describe("Request.body with Blob", () => {
    it("should return ReadableStream for POST request with Blob body", () => {
      const blob = new Blob(["blob content"], { type: "text/plain" });
      const request = new Request("https://example.com", {
        method: "POST",
        body: blob,
      });
      expect(request.body).toBeInstanceOf(ReadableStream);
    });

    it("should be able to read request body stream from Blob", async () => {
      const text = "Blob body content";
      const blob = new Blob([text], { type: "text/plain" });
      const request = new Request("https://example.com", {
        method: "POST",
        body: blob,
      });

      const reader = request.body!.getReader();
      const chunks: Uint8Array[] = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }

      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(text);
    });
  });

  describe("Response.clone() body access", () => {
    it("should allow reading body from cloned response", async () => {
      const text = "Original response body";
      const response = new Response(text);
      const cloned = response.clone();

      // Read from the clone
      const clonedText = await cloned.text();
      expect(clonedText).toEqual(text);
    });

    it("should allow reading body stream from cloned response", async () => {
      const text = "Cloned stream body";
      const response = new Response(text);
      const cloned = response.clone();

      // Read from clone's body stream
      const reader = cloned.body!.getReader();
      const chunks: Uint8Array[] = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }

      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(text);
    });

    it("should allow clone after accessing body (tee)", async () => {
      const text = "Body for tee test";
      const response = new Response(text);

      // Access body first to create the stream
      const _body = response.body;
      expect(_body).toBeInstanceOf(ReadableStream);

      // Now clone - this should use tee() internally
      const cloned = response.clone();

      // Both original and clone should be readable
      const [originalText, clonedText] = await Promise.all([
        response.text(),
        cloned.text(),
      ]);

      expect(originalText).toEqual(text);
      expect(clonedText).toEqual(text);
    });

    it("should allow clone after accessing body and read via streams (tee)", async () => {
      const text = "Stream tee test data";
      const response = new Response(text);

      // Access body first
      const originalBody = response.body;
      expect(originalBody).toBeInstanceOf(ReadableStream);

      // Clone after body access - uses tee()
      const cloned = response.clone();

      // Read both bodies via streams
      const readStream = async (stream: ReadableStream<Uint8Array>) => {
        const reader = stream.getReader();
        const chunks: Uint8Array[] = [];
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;
          chunks.push(value);
        }
        return new TextDecoder().decode(
          new Uint8Array(chunks.flatMap((c) => [...c]))
        );
      };

      const originalResult = await readStream(response.body!);
      const clonedResult = await readStream(cloned.body!);

      expect(originalResult).toEqual(text);
      expect(clonedResult).toEqual(text);
    });

    it("should allow multiple clones after body access", async () => {
      const text = "Multiple clone test";
      const response = new Response(text);

      // Access body
      const _body = response.body;

      // Clone multiple times
      const clone1 = response.clone();
      const clone2 = response.clone();

      // All three should be readable
      const [originalText, clone1Text, clone2Text] = await Promise.all([
        response.text(),
        clone1.text(),
        clone2.text(),
      ]);

      expect(originalText).toEqual(text);
      expect(clone1Text).toEqual(text);
      expect(clone2Text).toEqual(text);
    });
  });

  describe("Multiple chunks from stream", () => {
    it("should handle ReadableStream with multiple chunks", async () => {
      const chunks = ["chunk1", "chunk2", "chunk3"];
      const stream = new ReadableStream({
        start(controller) {
          for (const chunk of chunks) {
            controller.enqueue(new TextEncoder().encode(chunk));
          }
          controller.close();
        },
      });

      const request = new Request("https://example.com", {
        method: "POST",
        body: stream,
      });

      const result = await request.text();
      expect(result).toEqual(chunks.join(""));
    });

    it("should handle async ReadableStream with delayed chunks", async () => {
      const chunks = ["async1", "async2", "async3"];
      let chunkIndex = 0;

      const stream = new ReadableStream({
        async pull(controller) {
          if (chunkIndex < chunks.length) {
            // Simulate async delay
            await new Promise((resolve) => setTimeout(resolve, 10));
            controller.enqueue(new TextEncoder().encode(chunks[chunkIndex]));
            chunkIndex++;
          } else {
            controller.close();
          }
        },
      });

      const request = new Request("https://example.com", {
        method: "POST",
        body: stream,
      });

      const result = await request.text();
      expect(result).toEqual(chunks.join(""));
    });
  });

  describe("Large body streaming", () => {
    it("should handle large body data", async () => {
      // Create a 100KB string
      const largeText = "x".repeat(100 * 1024);
      const response = new Response(largeText);

      const reader = response.body!.getReader();
      const chunks: Uint8Array[] = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }

      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(largeText);
      expect(result.length).toEqual(100 * 1024);
    });
  });

  describe("Stream error handling", () => {
    it("should handle stream that errors", async () => {
      const stream = new ReadableStream({
        start(controller) {
          controller.enqueue(new TextEncoder().encode("partial"));
          controller.error(new Error("Stream error"));
        },
      });

      const request = new Request("https://example.com", {
        method: "POST",
        body: stream,
      });

      // Reading should eventually throw or return partial data
      try {
        await request.text();
        // If we get here, partial data was returned
      } catch (err: any) {
        // Error propagated - this is also valid behavior
        expect(err).toBeDefined();
      }
    });
  });

  describe("ArrayBuffer body", () => {
    it("should return ReadableStream for ArrayBuffer body", () => {
      const buffer = new ArrayBuffer(8);
      const view = new Uint8Array(buffer);
      view.set([1, 2, 3, 4, 5, 6, 7, 8]);

      const response = new Response(buffer);
      expect(response.body).toBeInstanceOf(ReadableStream);
    });

    it("should be able to read ArrayBuffer body stream", async () => {
      const buffer = new ArrayBuffer(4);
      const view = new Uint8Array(buffer);
      view.set([65, 66, 67, 68]); // "ABCD"

      const response = new Response(buffer);
      const reader = response.body!.getReader();
      const chunks: Uint8Array[] = [];
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;
        chunks.push(value);
      }

      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual("ABCD");
    });
  });

  describe("Byte stream controller (type: bytes)", () => {
    it("Response.body should use ReadableByteStreamController", () => {
      const response = new Response("test data");
      const body = response.body!;

      // Verify it's a ReadableStream
      expect(body).toBeInstanceOf(ReadableStream);

      // The stream should support BYOB reader (only byte streams do)
      // Getting a BYOB reader would throw on non-byte streams
      const reader = body.getReader({ mode: "byob" });
      expect(reader).toBeDefined();
      reader.releaseLock();
    });

    it("Request.body should use ReadableByteStreamController", () => {
      const request = new Request("https://example.com", {
        method: "POST",
        body: "test data",
      });
      const body = request.body!;

      // Verify it's a ReadableStream
      expect(body).toBeInstanceOf(ReadableStream);

      // The stream should support BYOB reader (only byte streams do)
      const reader = body.getReader({ mode: "byob" });
      expect(reader).toBeDefined();
      reader.releaseLock();
    });

    it("should be able to read with BYOB reader", async () => {
      const text = "Hello, BYOB!";
      const response = new Response(text);
      const body = response.body!;

      // Get BYOB reader
      const reader = body.getReader({ mode: "byob" });

      // Create a buffer to read into
      const buffer = new ArrayBuffer(text.length);
      let view = new Uint8Array(buffer);

      // Read into the provided buffer
      const result = await reader.read(view);

      expect(result.done).toBeFalsy();
      expect(result.value).toBeInstanceOf(Uint8Array);

      const decoder = new TextDecoder();
      const resultText = decoder.decode(result.value);
      expect(resultText).toEqual(text);

      reader.releaseLock();
    });
  });

  describe("pipeTo() with body streams", () => {
    it("should pipe Response body to WritableStream", async () => {
      const text = "Hello, pipeTo!";
      const response = new Response(text);

      const chunks: Uint8Array[] = [];
      const writable = new WritableStream({
        write(chunk) {
          chunks.push(chunk);
        },
      });

      await response.body!.pipeTo(writable);

      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(text);
    });

    it("should pipe Request body to WritableStream", async () => {
      const text = "Request body for pipeTo";
      const request = new Request("https://example.com", {
        method: "POST",
        body: text,
      });

      const chunks: Uint8Array[] = [];
      const writable = new WritableStream({
        write(chunk) {
          chunks.push(chunk);
        },
      });

      await request.body!.pipeTo(writable);

      const decoder = new TextDecoder();
      const result = chunks.map((chunk) => decoder.decode(chunk)).join("");
      expect(result).toEqual(text);
    });

    it("should pipe multiple chunks through pipeTo", async () => {
      const inputChunks = ["chunk1", "chunk2", "chunk3"];
      const stream = new ReadableStream({
        start(controller) {
          for (const chunk of inputChunks) {
            controller.enqueue(new TextEncoder().encode(chunk));
          }
          controller.close();
        },
      });

      const outputChunks: string[] = [];
      const writable = new WritableStream({
        write(chunk) {
          outputChunks.push(new TextDecoder().decode(chunk));
        },
      });

      await stream.pipeTo(writable);

      expect(outputChunks.join("")).toEqual(inputChunks.join(""));
    });

    it("should handle pipeTo with preventClose option", async () => {
      const text = "preventClose test";
      const response = new Response(text);

      let closeCalled = false;
      const writable = new WritableStream({
        write(_chunk) {},
        close() {
          closeCalled = true;
        },
      });

      await response.body!.pipeTo(writable, { preventClose: true });

      // With preventClose, the writable stream should not be closed
      expect(closeCalled).toBeFalsy();
    });
  });

  // Note: pipeThrough() tests are not included because TransformStream
  // is not yet implemented in LLRT. Once TransformStream is added,
  // pipeThrough() tests should be added here.
});
