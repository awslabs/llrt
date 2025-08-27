import { platform } from "os";
const IS_WINDOWS = platform() === "win32";

describe("Headers class", () => {
  it("should construct a new Headers object with the provided headers", () => {
    const headers = { "content-type": "application/json" };
    const h = new Headers(headers);
    expect(h.get("Content-Type")).toEqual(headers["content-type"]);
  });

  it("should add headers to the Headers object", () => {
    const h = new Headers();
    h.set("Content-Type", "application/json");
    expect(h.get("Content-Type")).toEqual("application/json");
  });

  it("should overwrite headers in the Headers object", () => {
    const headers = { "Content-Type": "application/json" };
    const h = new Headers(headers);
    h.set("Content-Type", "text/plain");
    expect(h.get("Content-Type")).toEqual("text/plain");
  });

  it("should delete headers from the Headers object", () => {
    const headers = { "Content-Type": "application/json" };
    const h = new Headers(headers);
    h.delete("Content-Type");
    expect(h.get("Content-Type")).toBeNull();
  });

  it("should return an iterator over the headers", () => {
    const headers = {
      "content-type": "application/json",
      authorization: "Bearer 1234",
    };
    const h = new Headers(headers);
    h.append("set-cookie", "AAA=123; expires=Sun, 10-Nov-2024 12:29:35 GMT");
    h.append("set-cookie", "BBB=456; expires=Sun, 10-Nov-2024 12:29:35 GMT");

    const iterator = h.entries();
    let next = iterator.next();
    expect(next.value).toStrictEqual(["authorization", "Bearer 1234"]);
    next = iterator.next();
    expect(next.value).toStrictEqual(["content-type", "application/json"]);
    next = iterator.next();
    expect(next.value).toStrictEqual([
      "set-cookie",
      "AAA=123; expires=Sun, 10-Nov-2024 12:29:35 GMT",
    ]);
    next = iterator.next();
    expect(next.value).toStrictEqual([
      "set-cookie",
      "BBB=456; expires=Sun, 10-Nov-2024 12:29:35 GMT",
    ]);
    next = iterator.next();
    expect(next.value).toStrictEqual(undefined);
  });

  it("should iterate over the headers with forEach", () => {
    const headers = {
      "content-type": "application/json",
    };
    const h = new Headers(headers);
    h.forEach((value, key) => {
      expect(key).toStrictEqual("content-type");
      expect(value).toStrictEqual("application/json");
    });
  });

  it("should be returned as array type of string", () => {
    const h = new Headers();
    h.append("set-cookie", "AAA=123; expires=Sun, 10-Nov-2024 12:29:35 GMT");
    h.append("set-cookie", "BBB=456; expires=Sun, 10-Nov-2024 12:29:35 GMT");
    expect(h.getSetCookie()).toStrictEqual([
      "AAA=123; expires=Sun, 10-Nov-2024 12:29:35 GMT",
      "BBB=456; expires=Sun, 10-Nov-2024 12:29:35 GMT",
    ]);
  });

  it("should be returned as a semicolon-delimited string", () => {
    const h = new Headers();
    h.append("cookie", "AAA=123");
    h.append("cookie", "BBB=456");
    expect(h.get("cookie")).toStrictEqual("AAA=123; BBB=456");
  });

  it("should be returned as a comma-delimited string", () => {
    const h = new Headers();
    h.append("accept-encoding", "zstd");
    h.append("accept-encoding", "br");
    expect(h.get("accept-encoding")).toStrictEqual("zstd, br");
  });
});

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
});

describe("Response class", () => {
  it("should construct a new Response object with default values", () => {
    const response = new Response();
    expect(response.status).toEqual(200);
    expect(response.statusText).toEqual("OK");
    expect(response.headers instanceof Headers).toBeTruthy();
    expect(response.body).toEqual(undefined);
    expect(response.redirected).toBeFalsy();
  });

  it("should set the status and statusText to the provided values", () => {
    const response = new Response(null, {
      status: 404,
      statusText: "Not Found",
    });
    expect(response.status).toEqual(404);
    expect(response.statusText).toEqual("Not Found");
  });

  it("should set the headers to the provided value", () => {
    const headers = new Headers({ "Content-Type": "application/json" });
    const response = new Response(null, { headers });

    expect(response.headers.get("Content-Type")).toStrictEqual(
      "application/json"
    );
  });

  it("should set the body to the provided value", async () => {
    const body = "Hello, world!";
    const response = new Response(body);
    expect(response.bodyUsed).toBeFalsy();
    expect(await response.text()).toStrictEqual(body);
    expect(response.bodyUsed).toBeTruthy();
  });

  it("should set the body to a Blob if a Blob is provided", async () => {
    const blob = new Blob(["Hello, world!"], { type: "text/plain" });
    const response = new Response(blob);
    expect(response.bodyUsed).toBeFalsy();
    expect(await response.text()).toEqual("Hello, world!");
    expect(response.bodyUsed).toBeTruthy();
  });

  it("should set the body to a JSON object if a JSON object is provided", async () => {
    const jsonBody = { key: "value" };
    const response = new Response(JSON.stringify(jsonBody), {
      headers: { "Content-Type": "application/json" },
    });
    expect(response.bodyUsed).toBeFalsy();
    expect(await response.json()).toStrictEqual(jsonBody);
    expect(response.bodyUsed).toBeTruthy();
  });

  it("should set the body to a bytes object if a bytes object is provided", async () => {
    const myArray = new Uint8Array([1, 2, 3]);
    const response = new Response(myArray);
    expect(response.bodyUsed).toBeFalsy();
    expect(await response.bytes()).toStrictEqual(myArray);
    expect(response.bodyUsed).toBeTruthy();
  });

  it("should clone the response with the clone() method", () => {
    const response = new Response("Original response");
    const clonedResponse = response.clone();
    expect(response.body).toEqual(clonedResponse.body);
    expect(response.url).toEqual(clonedResponse.url);
    expect(response.status).toEqual(clonedResponse.status);
    expect(response.statusText).toEqual(clonedResponse.statusText);
    expect(response.headers).toEqual(clonedResponse.headers);
    expect(response.type).toEqual(clonedResponse.type);
    expect(response.ok).toEqual(clonedResponse.ok);
    expect(response.bodyUsed).toEqual(clonedResponse.bodyUsed);
    expect(response.redirected).toEqual(clonedResponse.redirected);
  });

  it("should create a Response object with an ok status for status codes in the range 200-299", () => {
    const response = new Response("Success", { status: 204 });
    expect(response.ok).toBeTruthy();
  });

  it("should create a Response object with not-ok status for status codes outside the range 200-299", () => {
    const response = new Response("Error", { status: 404 });
    expect(!response.ok).toBeTruthy();
  });

  it("should be returned specified values in error static function", () => {
    const response = Response.error();
    expect(response.status).toEqual(0);
    expect(response.statusText).toEqual("");
    expect(response.headers instanceof Headers).toBeTruthy();
    expect(response.body).toEqual(undefined);
    expect(response.type).toEqual("error");
  });

  it("should be returned specified values in redirect static function called single param", () => {
    const redirectUrl = "http://localhost/";
    //@ts-ignore
    const response = Response.redirect(redirectUrl);
    expect(response.status).toEqual(302);
    expect(response.headers.get("location")).toEqual(redirectUrl);
  });

  it("should be returned specified values in redirect static function called double param", () => {
    const redirectUrl = "http://localhost/";
    const response = Response.redirect(redirectUrl, 301);
    expect(response.status).toEqual(301);
    expect(response.headers.get("location")).toEqual(redirectUrl);
  });

  it("should be returned specified values in json static function called single param", () => {
    const jsonBody = { some: "data", more: "information" };
    const response = Response.json(JSON.stringify(jsonBody));
    expect(response.status).toEqual(200);
    response.json().then((parsedJson) => {
      expect(parsedJson).toStrictEqual(jsonBody);
    });
  });

  it("should be returned specified values in json static function called double param", () => {
    const jsonBody = { some: "data", more: "information" };
    const response = Response.json(JSON.stringify(jsonBody), {
      status: 200,
      statusText: "SuperSmashingGreat!",
      headers: { "Content-Type": "application/json" },
    });
    expect(response.status).toEqual(200);
    expect(response.statusText).toEqual("SuperSmashingGreat!");
    response.json().then((parsedJson) => {
      expect(parsedJson).toStrictEqual(jsonBody);
    });
  });
});
