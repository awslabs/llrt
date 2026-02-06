describe("Response class", () => {
  it("should construct a new Response object with default values", () => {
    const response = new Response();
    expect(response.status).toEqual(200);
    expect(response.statusText).toEqual("OK");
    expect(response.headers instanceof Headers).toBeTruthy();
    expect(response.body).toEqual(null);
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
    const response = new Response(null, { status: 204 });
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
    expect(response.body).toEqual(null);
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

  it("returns JSON with 200 by default", async () => {
    const json = { message: "pong" };
    const res = Response.json(json);

    expect(res.status).toEqual(200);
    expect(res.headers.get("content-type")).toEqual(
      "application/json;charset=UTF-8"
    );
    expect(await res.json()).toEqual(json);
  });

  it("serializes JSON body to text", async () => {
    const json = { message: "pong" };
    const res = Response.json(json);

    expect(res.status).toEqual(200);
    expect(res.headers.get("content-type")).toEqual(
      "application/json;charset=UTF-8"
    );
    expect(await res.text()).toEqual(JSON.stringify(json));
  });

  it("supports custom status and statusText", async () => {
    const json = { some: "data", more: "information" };
    const res = Response.json(json, {
      status: 307,
      statusText: "Temporary Redirect",
    });
    expect(res.status).toEqual(307);
    expect(res.statusText).toEqual("Temporary Redirect");
    expect(res.headers.get("content-type")).toEqual(
      "application/json;charset=UTF-8"
    );
    expect(await res.text()).toEqual(JSON.stringify(json));
  });

  // ── Null body status: body with 204/304 should throw ──

  it("should throw when body is provided with 204 status", () => {
    expect(() => {
      new Response("body", { status: 204 });
    }).toThrow();
  });

  it("should throw when body is provided with 304 status", () => {
    expect(() => {
      new Response("body", { status: 304 });
    }).toThrow();
  });

  it("should allow null body with 204 status", () => {
    const res = new Response(null, { status: 204 });
    expect(res.status).toEqual(204);
  });

  it("should allow null body with 304 status", () => {
    const res = new Response(null, { status: 304 });
    expect(res.status).toEqual(304);
  });

  // ── Status range validation ──

  it("should throw RangeError for status below 200", () => {
    expect(() => {
      new Response(null, { status: 100 });
    }).toThrow();
  });

  it("should throw RangeError for status 600", () => {
    expect(() => {
      new Response(null, { status: 600 });
    }).toThrow();
  });

  // ── Body unusable: disturbed ──

  it("should reject text() after body has been consumed", async () => {
    const res = new Response("hello");
    await res.text();
    await expect(res.text()).rejects.toThrow();
  });

  it("should reject json() after text() has been called", async () => {
    const res = new Response(JSON.stringify({ a: 1 }));
    await res.text();
    await expect(res.json()).rejects.toThrow();
  });

  it("should reject arrayBuffer() after text()", async () => {
    const res = new Response("data");
    await res.text();
    await expect(res.arrayBuffer()).rejects.toThrow();
  });

  it("should reject blob() after text()", async () => {
    const res = new Response("data");
    await res.text();
    await expect(res.blob()).rejects.toThrow();
  });

  it("should reject bytes() after text()", async () => {
    const res = new Response("data");
    await res.text();
    await expect(res.bytes()).rejects.toThrow();
  });

  // ── Body unusable: locked ──
  // Note: These tests require body to be a ReadableStream, which only happens
  // for fetched responses in LLRT. User-provided bodies return raw values.

  it("should reject text() when body stream is locked", async () => {
    const res = new Response("locked test");
    // Skip if body is not a ReadableStream (LLRT returns raw value for user-provided bodies)
    if (!res.body || typeof res.body.getReader !== "function") {
      return;
    }
    const reader = res.body.getReader();
    await expect(res.text()).rejects.toThrow();
    reader.releaseLock();
  });

  it("should reject arrayBuffer() when body stream is locked", async () => {
    const res = new Response("locked test");
    // Skip if body is not a ReadableStream
    if (!res.body || typeof res.body.getReader !== "function") {
      return;
    }
    const reader = res.body.getReader();
    await expect(res.arrayBuffer()).rejects.toThrow();
    reader.releaseLock();
  });

  // ── Null body consumption ──

  it("should return empty string from text() on null body", async () => {
    const res = new Response(null);
    expect(await res.text()).toEqual("");
  });

  it("should return 0-length ArrayBuffer from arrayBuffer() on null body", async () => {
    const res = new Response(null);
    const buf = await res.arrayBuffer();
    expect(buf.byteLength).toEqual(0);
  });

  it("should return 0-length Uint8Array from bytes() on null body", async () => {
    const res = new Response(null);
    const bytes = await res.bytes();
    expect(bytes.byteLength).toEqual(0);
  });

  it("should return 0-size Blob from blob() on null body", async () => {
    const res = new Response(null);
    const blob = await res.blob();
    expect(blob.size).toEqual(0);
  });

  // ── clone() edge cases ──

  it("should throw when cloning a Response with used body", async () => {
    const res = new Response("data");
    await res.text();
    expect(() => res.clone()).toThrow();
  });

  it("should allow cloning a Response with null body", () => {
    const res = new Response(null);
    const cloned = res.clone();
    expect(cloned.status).toEqual(res.status);
  });

  it("should independently consume cloned bodies via different methods", async () => {
    const res = new Response("clone test");
    const cloned = res.clone();
    const text = await res.text();
    const buf = await cloned.arrayBuffer();
    expect(text).toEqual("clone test");
    expect(new TextDecoder().decode(buf)).toEqual("clone test");
  });

  // ── body/bodyUsed for null vs non-null ──

  it("should return null body for Response(null)", () => {
    const res = new Response(null);
    expect(res.body).toBeNull();
  });

  it("should return null body for Response(undefined)", () => {
    const res = new Response(undefined);
    expect(res.body).toBeNull();
  });

  it("should have non-null body for Response with empty string", () => {
    const res = new Response("");
    expect(res.body).not.toBeNull();
  });

  it("should report bodyUsed=false for null body", () => {
    const res = new Response(null);
    expect(res.bodyUsed).toBeFalsy();
  });

  // ── ReadableStream as Response body ──

  it("should accept ReadableStream as body", async () => {
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("stream"));
        controller.close();
      },
    });
    const res = new Response(stream);
    expect(await res.text()).toEqual("stream");
  });

  it("should reject disturbed ReadableStream as body", async () => {
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("data"));
        controller.close();
      },
    });
    const reader = stream.getReader();
    await reader.read();
    reader.releaseLock();
    expect(() => new Response(stream)).toThrow();
  });

  it("should reject locked ReadableStream as body", () => {
    const stream = new ReadableStream({
      start(controller) {
        controller.close();
      },
    });
    stream.getReader(); // locks
    expect(() => new Response(stream)).toThrow();
  });

  it("should set bodyUsed after consuming ReadableStream body", async () => {
    const stream = new ReadableStream({
      start(controller) {
        controller.enqueue(new TextEncoder().encode("test"));
        controller.close();
      },
    });
    const res = new Response(stream);
    expect(res.bodyUsed).toBeFalsy();
    await res.text();
    expect(res.bodyUsed).toBeTruthy();
  });

  // ── BodyInit type coercion ──

  it("should handle URLSearchParams as body", async () => {
    const params = new URLSearchParams({ foo: "bar", baz: "qux" });
    const res = new Response(params);
    expect(await res.text()).toEqual("foo=bar&baz=qux");
  });

  it("should handle ArrayBuffer as body", async () => {
    const buf = new TextEncoder().encode("arraybuffer").buffer;
    const res = new Response(buf);
    expect(await res.text()).toEqual("arraybuffer");
  });

  it("should handle DataView as body", async () => {
    const buf = new TextEncoder().encode("dataview").buffer;
    const view = new DataView(buf);
    const res = new Response(view);
    expect(new TextDecoder().decode(await res.arrayBuffer())).toEqual(
      "dataview"
    );
  });

  // ── Mixed body type consumption ──

  it("should read string body as arrayBuffer", async () => {
    const res = new Response("hello");
    const buf = await res.arrayBuffer();
    expect(new TextDecoder().decode(buf)).toEqual("hello");
  });

  it("should read string body as blob", async () => {
    const res = new Response("hello");
    const blob = await res.blob();
    expect(blob.size).toEqual(5);
    expect(await blob.text()).toEqual("hello");
  });

  it("should read Uint8Array body as json", async () => {
    const data = new TextEncoder().encode('{"key":"value"}');
    const res = new Response(data);
    expect(await res.json()).toEqual({ key: "value" });
  });

  it("should read Blob body as bytes", async () => {
    const blob = new Blob(["blob data"]);
    const res = new Response(blob);
    const bytes = await res.bytes();
    expect(new TextDecoder().decode(bytes)).toEqual("blob data");
  });

  // ── Response.json() edge cases ──

  it("should serialize number via Response.json()", async () => {
    expect(await Response.json(42).json()).toEqual(42);
  });

  it("should serialize null via Response.json()", async () => {
    expect(await Response.json(null).json()).toBeNull();
  });

  it("should serialize array via Response.json()", async () => {
    expect(await Response.json([1, 2, 3]).json()).toEqual([1, 2, 3]);
  });

  it("should serialize boolean via Response.json()", async () => {
    expect(await Response.json(true).json()).toEqual(true);
  });

  it("should not override custom content-type in Response.json() init", () => {
    const res = Response.json(
      {},
      { headers: { "Content-Type": "application/feed+json" } }
    );
    expect(res.headers.get("content-type")).toEqual("application/feed+json");
  });

  it("should throw for non-serializable value in Response.json()", () => {
    expect(() => Response.json(BigInt(42))).toThrow();
  });

  // ── Response.redirect() edge cases ──

  it("should throw RangeError for non-redirect status in redirect()", () => {
    expect(() => Response.redirect("http://localhost", 200)).toThrow();
  });

  it("should accept 301 in redirect()", () => {
    expect(Response.redirect("http://localhost", 301).status).toEqual(301);
  });

  it("should accept 307 in redirect()", () => {
    expect(Response.redirect("http://localhost", 307).status).toEqual(307);
  });

  it("should accept 308 in redirect()", () => {
    expect(Response.redirect("http://localhost", 308).status).toEqual(308);
  });

  it("should have null body on redirect()", () => {
    expect(Response.redirect("http://localhost", 302).body).toBeNull();
  });
});
