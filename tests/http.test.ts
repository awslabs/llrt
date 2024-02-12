describe("Headers", () => {
  it("should construct a new Headers object with the provided headers", () => {
    const headers = { "content-type": "application/json" };
    const h = new Headers(headers);
    assert.strictEqual(h.get("Content-Type"), headers["content-type"]);
  });

  it("should add headers to the Headers object", () => {
    const h = new Headers();
    h.set("Content-Type", "application/json");
    assert.strictEqual(h.get("Content-Type"), "application/json");
  });

  it("should overwrite headers in the Headers object", () => {
    const headers = { "Content-Type": "application/json" };
    const h = new Headers(headers);
    h.set("Content-Type", "text/plain");
    assert.strictEqual(h.get("Content-Type"), "text/plain");
  });

  it("should delete headers from the Headers object", () => {
    const headers = { "Content-Type": "application/json" };
    const h = new Headers(headers);
    h.delete("Content-Type");
    assert.strictEqual(h.get("Content-Type"), undefined);
  });

  it("should return an iterator over the headers", () => {
    const headers = {
      "content-type": "application/json",
      authorization: "Bearer 1234",
    };
    const h = new Headers(headers);
    const iterator = h.entries();
    let next = iterator.next();
    assert.deepStrictEqual(next.value, ["authorization", "Bearer 1234"]);
    next = iterator.next();
    assert.deepStrictEqual(next.value, ["content-type", "application/json"]);
    next = iterator.next();
    assert.deepStrictEqual(next.value, undefined);
  });
});

describe("Request", () => {
  it("should construct a new Request object with the provided URL", () => {
    const url = "https://example.com";
    const request = new Request(url);
    assert.strictEqual(request.url, url);
  });

  it("should set the method to GET by default", () => {
    const request = new Request("https://example.com");
    assert.strictEqual(request.method, "GET");
  });

  it("should set the method to the provided value", () => {
    const method = "POST";
    const request = new Request("https://example.com", { method });
    assert.strictEqual(request.method, method);
  });

  it("should set the headers to an empty object by default", () => {
    const request = new Request("https://example.com");
    const headers = new Headers();
    assert.deepEqual(request.headers.entries(), headers.entries());
  });

  it("should set the headers to the provided value", () => {
    const headers = { "Content-Type": "application/json" };
    const headerValue = new Headers(headers);
    const request = new Request("https://example.com", { headers });
    assert.deepStrictEqual(request.headers, headerValue);
  });

  it("should set the body to undefined by default", () => {
    const request = new Request("https://example.com");
    assert.strictEqual(request.body, undefined);
  });

  it("should set the body to the provided value", () => {
    const body = "hello world!";
    const request = new Request("https://example.com", { body });
    assert.deepStrictEqual(request.body, body);
  });

  it("should accept another request object as argument", () => {
    const oldRequest = new Request("https://example.com", {
      headers: { From: "webmaster@example.org" },
    });
    assert.equal(oldRequest.headers.get("From"), "webmaster@example.org");
    const newRequest = new Request(oldRequest, {
      headers: { From: "developer@example.org" },
    });
    assert.equal(newRequest.url, "https://example.com");
    assert.equal(newRequest.headers.get("From"), "developer@example.org");
  });
});

describe("Response class", () => {
  it("should construct a new Response object with default values", () => {
    const response = new Response();
    assert.strictEqual(response.status, 200);
    assert.strictEqual(response.statusText, "OK");
    assert.ok(response.headers instanceof Headers);
    assert.strictEqual(response.body, null);
  });

  it("should set the status and statusText to the provided values", () => {
    const response = new Response(null, {
      status: 404,
      statusText: "Not Found",
    });
    assert.strictEqual(response.status, 404);
    assert.strictEqual(response.statusText, "Not Found");
  });

  it("should set the headers to the provided value", () => {
    const headers = new Headers({ "Content-Type": "application/json" });
    const response = new Response(null, { headers });
    assert.deepStrictEqual(
      response.headers.get("Content-Type"),
      "application/json"
    );
  });

  it("should set the body to the provided value", () => {
    const body = "Hello, world!";
    const response = new Response(body);
    assert.deepStrictEqual(response.body, body);
  });

  it("should set the body to null if null is provided", () => {
    const response = new Response(null);
    assert.strictEqual(response.body, null);
  });

  it("should set the body to a Blob if a Blob is provided", () => {
    const blob = new Blob(["Hello, world!"], { type: "text/plain" });
    const response = new Response(blob);
    return response.text().then((text) => {
      assert.strictEqual(text, "Hello, world!");
    });
  });

  it("should set the body to a JSON object if a JSON object is provided", () => {
    const jsonBody = { key: "value" };
    const response = new Response(JSON.stringify(jsonBody), {
      headers: { "Content-Type": "application/json" },
    });
    return response.json().then((parsedJson) => {
      assert.deepStrictEqual(parsedJson, jsonBody);
    });
  });

  it("should clone the response with the clone() method", () => {
    const response = new Response("Original response");
    const clonedResponse = response.clone();
    assert.strictEqual(response.body, clonedResponse.body);
    assert.notStrictEqual(response, clonedResponse);
  });

  it("should create a Response object with an ok status for status codes in the range 200-299", () => {
    const response = new Response("Success", { status: 204 });
    assert.ok(response.ok);
  });

  it("should create a Response object with not-ok status for status codes outside the range 200-299", () => {
    const response = new Response("Error", { status: 404 });
    assert.ok(!response.ok);
  });
});

describe("URL class", () => {
  it("should parse a valid URL", () => {
    const url = new URL("https://www.example.com");
    assert.strictEqual(url.protocol, "https:");
    assert.strictEqual(url.hostname, "www.example.com");
  });

  it("should create a copy of a valid URL", () => {
    const url: any = new URL("https://www.example.com");
    const url2 = new URL(url);
    assert.notEqual(url, url2);
  });

  it("should to append base to a url", () => {
    const url = new URL("/base", "https://www.example.com");
    assert.equal(url.toString(), "https://www.example.com/base");
  });

  it("should throw an error for an invalid URL", () => {
    assert.throws(() => {
      new URL("not-a-url");
    }, /Invalid URL/);
  });

  it("should return the URL as a string", () => {
    const url = new URL("https://www.example.com");
    assert.strictEqual(url.toString(), "https://www.example.com/");
  });

  it("should parse query parameters", () => {
    const url = new URL("https://www.example.com/?foo=bar&baz=qux");
    assert.strictEqual(url.searchParams.get("foo"), "bar");
    assert.strictEqual(url.searchParams.get("baz"), "qux");
  });

  it("should modify query parameters", () => {
    const url = new URL("https://www.example.com/?foo=bar&baz=qux");
    url.searchParams.set("foo", "new-value");
    assert.strictEqual(
      url.toString(),
      "https://www.example.com/?baz=qux&foo=new-value"
    );
  });
});

describe("URLSearchParams class", () => {
  it("should have a parameter if it exists", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    assert.ok(searchParams.has("topic"));
  });

  it("should not have a parameter if it doesn't exist", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    assert.ok(!searchParams.has("foo"));
  });

  it("should return the value of the parameter if it exists", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    assert.ok(searchParams.get("topic") === "api");
  });

  it("should return null if the parameter doesn't exist", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    assert.equal(searchParams.get("foo"), null);
  });

  it("should return an array of all values of the parameter if it exists", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    assert.deepEqual(searchParams.getAll("a"), ["1", "2", "3"]);
  });

  it("should return an empty array if the parameter doesn't exist", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    assert.deepEqual(searchParams.getAll("foo"), []);
  });

  it("should add the parameter to the end of the query string", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    searchParams.append("topic", "webdev");
    assert.equal(searchParams.toString(), "a=1&a=2&a=3&topic=api&topic=webdev");
  });

  it("should replace all values of the parameter with the given value", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    searchParams.set("topic", "More webdev");
    assert.equal(searchParams.toString(), "a=1&a=2&a=3&topic=More+webdev");
  });

  it("should remove the parameter from the query string", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    searchParams.delete("topic");
    assert.equal(searchParams.toString(), "a=1&a=2&a=3");
  });

  it("should iterate over all parameters in the query string", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    let arr: [string, string][] = [];
    for (const p of searchParams) {
      arr.push(p);
    }
    assert.deepEqual(arr, [
      ["a", "1"],
      ["a", "2"],
      ["a", "3"],
      ["topic", "api"],
    ]);
  });
});
