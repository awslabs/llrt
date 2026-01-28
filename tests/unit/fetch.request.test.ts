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
    // Per WHATWG Fetch spec, body returns a ReadableStream
    expect(request.body).toBeInstanceOf(ReadableStream);
    // bodyUsed becomes true when body stream is accessed
    expect(request.bodyUsed).toBeTruthy();
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
});
