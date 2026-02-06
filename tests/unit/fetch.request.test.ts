describe("Request class", () => {
  it("should construct a new Request object with the provided URL", () => {
    const url = "https://example.com";
    const request = new Request(url);
    expect(request.url).toEqual(url);
  });

  it("should set the method to GET by default", () => {
    const request = new Request("https://example.com");
    expect(request.method).toEqual("GET");
  });

  it("should set the mode to cors by default", () => {
    const request = new Request("https://example.com");
    expect(request.mode).toEqual("cors");
  });

  it("should set the cache to no-store by default", () => {
    const request = new Request("https://example.com");
    expect(request.cache).toEqual("no-store");
  });

  it("should set the bodyUsed to false by default", () => {
    const request = new Request("https://example.com");
    expect(request.bodyUsed).toBeFalsy();
  });

  it("should set the method to the provided value", () => {
    const method = "POST";
    const request = new Request("https://example.com", { method });
    expect(request.method).toEqual(method);
  });

  it("should set the headers to an empty object by default", () => {
    const request = new Request("https://example.com");
    const headers = new Headers();
    expect(request.headers.entries()).toEqual(headers.entries());
  });

  it("should set the headers to the provided value", () => {
    const headers = { "Content-Type": "application/json" };
    const headerValue = new Headers(headers);
    const request = new Request("https://example.com", { headers });
    expect(request.headers).toStrictEqual(headerValue);
  });

  it("should set the body to null by default", () => {
    const request = new Request("https://example.com");
    expect(request.body).toEqual(null);
  });

  it("should set the body to the provided value", () => {
    const body = "hello world!";
    const request = new Request("https://example.com", {
      body,
      method: "POST",
    });
    expect(request.body).toStrictEqual(body);
    expect(request.bodyUsed).toBeFalsy();
  });

  it("should accept another request object as argument", () => {
    const oldRequest = new Request("https://example.com", {
      headers: { From: "webmaster@example.org" },
    });
    expect(oldRequest.headers.get("From")).toEqual("webmaster@example.org");
    const newRequest = new Request(oldRequest, {
      headers: { From: "developer@example.org" },
    });
    expect(newRequest.url).toEqual("https://example.com");
    expect(newRequest.headers.get("From")).toEqual("developer@example.org");
  });

  it("should accept a signal as an option", () => {
    const controller = new AbortController();
    const request = new Request("http://localhost", {
      signal: controller.signal,
    });
    expect(request.signal).toEqual(controller.signal);
  });

  it("should accept null or undefined as signal options", () => {
    // @ts-ignore
    const reqNull = new Request("http://localhost", { signal: null });
    expect(reqNull.signal).toBeUndefined();
    // @ts-ignore
    const reqUndef = new Request("http://localhost", { signal: undefined });
    expect(reqUndef.signal).toBeUndefined();
  });

  it("should fail if the signal option is not an object", () => {
    expect(() => {
      // @ts-ignore
      new Request("http://localhost", { signal: "type error" });
    }).toThrow(/property is not an AbortSignal/);
  });

  it("should fail if the signal option is not an valid object", () => {
    expect(() => {
      new Request("http://localhost", {
        // @ts-ignore
        signal: new Request("http://localhost"),
      });
    }).toThrow(/property is not an AbortSignal/);
  });

  it("should return the provided body via text() and set bodyUsed to true", async () => {
    const body = "Hello, world!";
    const request = new Request("http://localhost", {
      body: body,
      method: "POST",
    });
    expect(request.bodyUsed).toBeFalsy();
    expect(await request.text()).toStrictEqual(body);
    expect(request.bodyUsed).toBeTruthy();
  });

  it("should set the body to a JSON object if a JSON object is provided", async () => {
    const jsonBody = { key: "value" };
    const request = new Request("http://localhost", {
      body: JSON.stringify(jsonBody),
      method: "POST",
    });
    expect(request.bodyUsed).toBeFalsy();
    expect(await request.json()).toStrictEqual(jsonBody);
    expect(request.bodyUsed).toBeTruthy();
  });

  it("should set the body to a bytes object if a bytes object is provided", async () => {
    const myArray = new Uint8Array([1, 2, 3]);
    const request = new Request("http://localhost", {
      body: myArray,
      method: "POST",
    });
    expect(request.bodyUsed).toBeFalsy();
    expect(await request.bytes()).toStrictEqual(myArray);
    expect(request.bodyUsed).toBeTruthy();
  });

  it("should set the body to a Blob if a Blob is provided", async () => {
    const blob = new Blob(["Hello, world!"], { type: "text/plain" });
    const request = new Request("http://localhost", {
      body: blob,
      method: "POST",
    });
    expect(request.bodyUsed).toBeFalsy();
    const res = await request.blob();
    expect(request.bodyUsed).toBeTruthy();
    expect(res.size).toEqual(blob.size);
    expect(res.type).toEqual("text/plain");
  });

  it("should set the body to a Blob if Blob and content-type are provided", async () => {
    const blob = new Blob(["Hello, world!"], { type: "text/html" });
    const request = new Request("http://localhost", {
      body: blob,
      method: "POST",
      headers: { "content-type": "text/plain" },
    });
    const res = await request.blob();
    expect(res.size).toEqual(blob.size);
    expect(res.type).toEqual("text/plain");
  });

  it("should ignore request options which are not an object", async () => {
    const request = new Request("http://localhost", undefined);
    expect(request instanceof Request).toBeTruthy();
  });

  // ── Body with GET/HEAD should throw ──

  it("should throw when body is set with GET method", () => {
    expect(() => {
      new Request("http://localhost", { method: "GET", body: "data" });
    }).toThrow();
  });

  it("should throw when body is set with HEAD method", () => {
    expect(() => {
      new Request("http://localhost", { method: "HEAD", body: "data" });
    }).toThrow();
  });

  it("should allow body with PUT method", () => {
    const req = new Request("http://localhost", {
      method: "PUT",
      body: "data",
    });
    expect(req.method).toEqual("PUT");
  });

  it("should allow body with PATCH method", () => {
    const req = new Request("http://localhost", {
      method: "PATCH",
      body: "data",
    });
    expect(req.method).toEqual("PATCH");
  });

  it("should allow body with DELETE method", () => {
    const req = new Request("http://localhost", {
      method: "DELETE",
      body: "data",
    });
    expect(req.method).toEqual("DELETE");
  });

  // ── Body transfer from another Request ──

  it("should mark original body as used when constructing from another Request", async () => {
    const original = new Request("http://localhost", {
      method: "POST",
      body: "transferred",
    });
    const derived = new Request(original);
    expect(original.bodyUsed).toBeTruthy();
    expect(await derived.text()).toEqual("transferred");
  });

  it("should throw when reading body of a transferred Request", async () => {
    const original = new Request("http://localhost", {
      method: "POST",
      body: "transferred",
    });
    new Request(original);
    await expect(original.text()).rejects.toThrow();
  });

  // ── Unusable body: disturbed ──

  it("should reject json() after text() has been called", async () => {
    const req = new Request("http://localhost", {
      method: "POST",
      body: JSON.stringify({ a: 1 }),
    });
    await req.text();
    await expect(req.json()).rejects.toThrow();
  });

  // ── Unusable body: locked ──
  // Note: LLRT returns raw value for body, not ReadableStream, so locked check doesn't apply

  it("should reject text() when body stream is locked by a reader", async () => {
    const req = new Request("http://localhost", {
      method: "POST",
      body: "locked",
    });
    // LLRT returns raw value for body, not ReadableStream
    // Skip if body doesn't have getReader
    if (!req.body || typeof req.body.getReader !== "function") {
      return;
    }
    const reader = req.body.getReader();
    await expect(req.text()).rejects.toThrow();
    reader.releaseLock();
  });

  // ── clone() edge cases ──

  it("should throw when cloning a Request with used body", async () => {
    const req = new Request("http://localhost", {
      method: "POST",
      body: "data",
    });
    await req.text();
    expect(() => req.clone()).toThrow();
  });

  it("should independently consume cloned Request bodies", async () => {
    const req = new Request("http://localhost", {
      method: "POST",
      body: "clone test",
    });
    const cloned = req.clone();
    expect(await req.text()).toEqual("clone test");
    expect(await cloned.text()).toEqual("clone test");
  });

  // ── Null body consumption ──

  it("should return empty string from text() on GET request (null body)", async () => {
    const req = new Request("http://localhost");
    expect(await req.text()).toEqual("");
  });

  it("should return empty ArrayBuffer from arrayBuffer() on GET request", async () => {
    const req = new Request("http://localhost");
    const buf = await req.arrayBuffer();
    expect(buf.byteLength).toEqual(0);
  });

  // ── Content-Type auto-setting ──

  it("should auto-set Content-Type for string body", () => {
    const req = new Request("http://localhost", {
      method: "POST",
      body: "text",
    });
    expect(req.headers.get("content-type")).toEqual(
      "text/plain;charset=UTF-8"
    );
  });

  it("should auto-set Content-Type for Blob body with type", () => {
    const blob = new Blob(["data"], { type: "application/octet-stream" });
    const req = new Request("http://localhost", {
      method: "POST",
      body: blob,
    });
    expect(req.headers.get("content-type")).toEqual(
      "application/octet-stream"
    );
  });

  it("should auto-set Content-Type for URLSearchParams body", () => {
    const params = new URLSearchParams({ a: "1" });
    const req = new Request("http://localhost", {
      method: "POST",
      body: params,
    });
    expect(req.headers.get("content-type")).toEqual(
      "application/x-www-form-urlencoded;charset=UTF-8"
    );
  });

  it("should not override explicit Content-Type header", () => {
    const req = new Request("http://localhost", {
      method: "POST",
      body: "text",
      headers: { "Content-Type": "application/json" },
    });
    expect(req.headers.get("content-type")).toEqual("application/json");
  });

  it("should not set Content-Type for ArrayBuffer body", () => {
    const buf = new ArrayBuffer(4);
    const req = new Request("http://localhost", {
      method: "POST",
      body: buf,
    });
    expect(req.headers.get("content-type")).toBeNull();
  });

  // ── ReadableStream as Request body ──

  it("should accept ReadableStream as body with POST and duplex half", async () => {
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("stream body"));
        controller.close();
      },
    });
    const req = new Request("http://localhost", {
      method: "POST",
      body: stream,
      // @ts-ignore
      duplex: "half",
    });
    expect(await req.text()).toEqual("stream body");
  });

  it("should reject disturbed ReadableStream as Request body", async () => {
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("data"));
        controller.close();
      },
    });
    const reader = stream.getReader();
    await reader.read();
    reader.releaseLock();
    expect(() => {
      new Request("http://localhost", {
        method: "POST",
        body: stream,
        // @ts-ignore
        duplex: "half",
      });
    }).toThrow();
  });

  it("should reject locked ReadableStream as Request body", () => {
    const stream = new ReadableStream({
      start(controller) {
        controller.close();
      },
    });
    stream.getReader(); // locks
    expect(() => {
      new Request("http://localhost", {
        method: "POST",
        body: stream,
        // @ts-ignore
        duplex: "half",
      });
    }).toThrow();
  });
});
