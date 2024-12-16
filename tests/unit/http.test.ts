import * as urlModule from "url";
import { platform } from "os";
const IS_WINDOWS = platform() === "win32";

describe("URL module import", () => {
  it("global URL and imported URL are equal", () => {
    const testUrl = "https://www.example.com";
    const moduleUrl = new urlModule.URL(testUrl);
    const globalUrl = new URL(testUrl);
    expect(moduleUrl).toEqual(globalUrl);
  });
  it("global URLSearchParams and imported URLSearchParams are equal", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const moduleSearchParams = new urlModule.URLSearchParams(paramsString);
    const globalSearchParams = new URLSearchParams(paramsString);
    expect(moduleSearchParams).toEqual(globalSearchParams);
  });
  describe("import { URL } from 'url';", () => {
    it("should parse a url hostname", () => {
      const testUrl = new urlModule.URL("https://www.example.com");
      expect(testUrl.protocol).toEqual("https:");
      expect(testUrl.host).toEqual("www.example.com");
      expect(testUrl.hostname).toEqual("www.example.com");
    });
    it("toString method works", () => {
      const testUrl = new urlModule.URL("/base", "https://www.example.com");
      expect(testUrl.toString()).toEqual("https://www.example.com/base");
    });
    it("canParse method works", () => {
      const validCanParse = urlModule.URL.canParse("https://www.example.com");
      const invalidCanParse = urlModule.URL.canParse("not_valid");
      expect(validCanParse).toEqual(true);
      expect(invalidCanParse).toEqual(false);
      expect(urlModule.URL.canParse("/foo", "https://example.org/")).toEqual(
        true
      );
    });
  });

  describe("import { URLSearchParams } from 'url';", () => {
    it("supports URLSearchParams basic API", () => {
      const paramsString = "topic=api&a=1&a=2&a=3";
      const searchParams = new urlModule.URLSearchParams(paramsString);
      expect(searchParams.size).toEqual(4);
      searchParams.append("foo", "bar");
      expect(searchParams.size).toEqual(5);
      expect(searchParams.has("topic")).toBeTruthy();
      expect(searchParams.has("foo")).toBeTruthy();
      searchParams.delete("foo");
      expect(searchParams.size).toEqual(4);
      expect(searchParams.has("foo")).toBeFalsy();
      expect(searchParams.get("topic")).toEqual("api");
      expect(searchParams.getAll("a")).toEqual(["1", "2", "3"]);
      searchParams.set("topic", "node");
      expect(searchParams.size).toEqual(4);
      expect(searchParams.get("topic")).toEqual("node");
    });
  });
});

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

  it("should set the mode to navigate by default", () => {
    const request = new Request("https://example.com");
    expect(request.mode).toEqual("navigate");
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
    expect(request.bodyUsed).toBeTruthy();
  });

  it("should set the body to a Blob if a Blob is provided", async () => {
    const blob = new Blob(["Hello, world!"], { type: "text/plain" });
    const request = new Request("https://example.com", {
      body: blob,
      method: "POST",
    });
    expect(request.body).toStrictEqual(
      new Uint8Array(await blob.arrayBuffer())
    );
    expect(request.body).toStrictEqual(await blob.bytes());
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

  it("should set the body to the provided value", async () => {
    const body = "Hello, world!";
    const request = new Request("http://localhost", {
      body: body,
      method: "POST",
    });
    expect(await request.text()).toStrictEqual(body);
    expect(request.bodyUsed).toBeTruthy();
  });

  it("should set the body to a JSON object if a JSON object is provided", async () => {
    const jsonBody = { key: "value" };
    const request = new Request("http://localhost", {
      body: JSON.stringify(jsonBody),
      method: "POST",
    });
    expect(await request.json()).toStrictEqual(jsonBody);
  });

  it("should set the body to a bytes object if a bytes object is provided", async () => {
    const myArray = new Uint8Array([1, 2, 3]);
    const request = new Request("http://localhost", {
      body: myArray,
      method: "POST",
    });
    expect(await request.bytes()).toStrictEqual(myArray);
  });

  it("should set the body to a Blob if a Blob is provided", async () => {
    const blob = new Blob(["Hello, world!"], { type: "text/plain" });
    const request = new Request("http://localhost", {
      body: blob,
      method: "POST",
    });
    expect(await request.text()).toEqual("Hello, world!");
    expect((await request.blob()).size).toEqual(blob.size);
    expect((await request.blob()).type).toEqual("");
  });

  it("should set the body to a Blob if Blob and content-type are provided", async () => {
    const blob = new Blob(["Hello, world!"], { type: "text/html" });
    const request = new Request("http://localhost", {
      body: blob,
      method: "POST",
      headers: { "content-type": "text/plain" },
    });
    expect(await request.text()).toEqual("Hello, world!");
    expect((await request.blob()).size).toEqual(blob.size);
    expect((await request.blob()).type).toEqual("text/plain");
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
    expect(await response.text()).toStrictEqual(body);
  });

  it("should set the body to a Blob if a Blob is provided", async () => {
    const blob = new Blob(["Hello, world!"], { type: "text/plain" });
    const response = new Response(blob);
    expect(await response.text()).toEqual("Hello, world!");
  });

  it("should set the body to a JSON object if a JSON object is provided", async () => {
    const jsonBody = { key: "value" };
    const response = new Response(JSON.stringify(jsonBody), {
      headers: { "Content-Type": "application/json" },
    });
    expect(await response.json()).toStrictEqual(jsonBody);
  });

  it("should set the body to a bytes object if a bytes object is provided", async () => {
    const myArray = new Uint8Array([1, 2, 3]);
    const response = new Response(myArray);
    expect(await response.bytes()).toStrictEqual(myArray);
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
});

describe("URL class", () => {
  it("should parse a valid URL", () => {
    const url = new URL("https://www.example.com");
    expect(url.protocol).toEqual("https:");
    expect(url.hostname).toEqual("www.example.com");
  });

  it("should create a copy of a valid URL", () => {
    const url: any = new URL("https://www.example.com");
    const url2 = new URL(url);
    expect(url).toEqual(url2);
    expect(url).not.toBe(url2);
  });

  it("should to append base to a url", () => {
    const url = new URL("/base", "https://www.example.com");
    expect(url.toString()).toEqual("https://www.example.com/base");
  });

  it("should throw an error for an invalid URL", () => {
    expect(() => {
      new URL("not-a-url");
    }).toThrow(/Invalid URL/);
  });

  it("should return the URL as a string", () => {
    const url = new URL("https://www.example.com");
    expect(url.toString()).toEqual("https://www.example.com/");
  });

  it("should parse query parameters", () => {
    const url = new URL("https://www.example.com/?foo=bar&baz=qux");
    expect(url.searchParams.get("foo")).toEqual("bar");
    expect(url.searchParams.get("baz")).toEqual("qux");
  });

  it("should be able to set and get port", () => {
    let url: any = new URL("https://www.example.com");
    url.port = "1234";
    expect(url.toString()).toEqual("https://www.example.com:1234/");
    url.port = 5678;
    expect(url.toString()).toEqual("https://www.example.com:5678/");

    url = new URL("https://www.example.com:443/route/example");
    expect(url.port).toEqual("");
    expect(url.toString()).toEqual("https://www.example.com/route/example");

    url.port = 21;
    expect(url.toString()).toEqual("https://www.example.com:21/route/example");

    url.protocol = "ftp";
    expect(url.toString()).toEqual("ftp://www.example.com/route/example");

    url.protocol = "http";
    url.port = 80;
    expect(url.toString()).toEqual("http://www.example.com/route/example");
  });

  it("should modify query parameters", () => {
    const url = new URL("https://www.example.com/?foo=bar&baz=qux");
    url.searchParams.set("foo", "new-value");
    expect(url.toString()).toEqual(
      "https://www.example.com/?foo=new-value&baz=qux"
    );
  });
  it("should parse username and password", () => {
    const url = new URL(
      "https://anonymous:flabada@developer.mozilla.org/en-US/docs/Web/API/URL/username"
    );
    expect(url.username).toEqual("anonymous");
    expect(url.password).toEqual("flabada");
  });
  it("should provide parse/canParse util", () => {
    const validUrl = "https://www.example.com/";
    const invalidUrl = "not_a_valid_url";
    expect(URL.parse(validUrl).href).toEqual(validUrl);
    expect(URL.canParse(validUrl)).toEqual(true);
    expect(URL.parse(invalidUrl)).toBeNull();
    expect(URL.canParse(invalidUrl)).toEqual(false);
  });
  it("parse/canParse works for relative urls", () => {
    expect(URL.parse("/foo", "https://example.org/").href).toEqual(
      "https://example.org/foo"
    );
    expect(URL.canParse("/foo", "https://example.org/")).toEqual(true);
  });
});

describe("URLSearchParams class", () => {
  it("constructor from array", () => {
    //@ts-ignore
    const searchParams = new URLSearchParams([["topic", ["api", "1234"]]]);
    expect(searchParams.get("topic")).toBe("api,1234");
  });

  it("constructor from object", () => {
    const paramsObject = { foo: "1", bar: "2" };
    const searchParams = new URLSearchParams(paramsObject);
    expect(searchParams.get("foo")).toBe("1");
    expect(searchParams.get("bar")).toBe("2");
  });

  it("constructor from iterator", () => {
    const paramsArray = [
      ["foo", "1"],
      ["bar", "2"],
    ];
    const paramsIterator = paramsArray[Symbol.iterator]();
    //@ts-ignore
    const searchParams = new URLSearchParams(paramsIterator);
    expect(searchParams.get("foo")).toBe("1");
    expect(searchParams.get("bar")).toBe("2");
  });

  it("constructor from string with special characters", () => {
    const paramsString = "topic=api&category=coding%20skills";
    const searchParams = new URLSearchParams(paramsString);
    expect(searchParams.get("topic")).toBe("api");
    expect(searchParams.get("category")).toBe("coding skills");
  });

  it("constructor from URLSearchParams object", () => {
    const existingParamsString = "topic=api";
    const existingSearchParams = new URLSearchParams(existingParamsString);
    const newSearchParams = new URLSearchParams(existingSearchParams);
    expect(newSearchParams.get("topic")).toBe("api");
  });

  it("should have a parameter if it exists", () => {
    const paramsString = "?topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    expect(searchParams.has("topic")).toBeTruthy();
  });

  it("should not have a parameter if it doesn't exist", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    expect(!searchParams.has("foo")).toBeTruthy();
  });

  it("should return the value of the parameter if it exists", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    expect(searchParams.get("topic") === "api").toBeTruthy();
  });

  it("should return null if the parameter doesn't exist", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    expect(searchParams.get("foo")).toBeNull();
  });

  it("should return an array of all values of the parameter if it exists", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    expect(searchParams.getAll("a")).toEqual(["1", "2", "3"]);
  });

  it("should return an empty array if the parameter doesn't exist", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    expect(searchParams.getAll("foo")).toEqual([]);
  });

  it("should add the parameter to the end of the query string", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    searchParams.append("topic", "webdev");
    expect(searchParams.toString()).toEqual(
      "topic=api&a=1&a=2&a=3&topic=webdev"
    );
  });

  it("should replace all values of the parameter with the given value", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    searchParams.set("topic", "More webdev");
    expect(searchParams.toString()).toEqual("topic=More+webdev&a=1&a=2&a=3");
  });

  it("should set value even if not existing", () => {
    const searchParams = new URLSearchParams("?bar=baz");
    searchParams.set("foo", "bar");
    expect(searchParams.toString()).toEqual("bar=baz&foo=bar");
  });

  it("should remove the parameter from the query string", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    searchParams.delete("topic");
    expect(searchParams.toString()).toEqual("a=1&a=2&a=3");
  });

  it("should iterate over all parameters in the query string", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    let arr: [string, string][] = [];
    for (const p of searchParams) {
      arr.push(p);
    }
    expect(arr).toEqual([
      ["topic", "api"],
      ["a", "1"],
      ["a", "2"],
      ["a", "3"],
    ]);
  });

  it("should for_each all parameters in the query string", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const searchParams = new URLSearchParams(paramsString);
    let arr: [string, string][] = [];
    searchParams.forEach((value, key) => {
      arr.push([key, value]);
    });
    expect(arr).toEqual([
      ["topic", "api"],
      ["a", "1"],
      ["a", "2"],
      ["a", "3"],
    ]);
  });

  it("should get key parameters in the query string", () => {
    const paramsString = "key1=value1&key2=value2";
    const searchParams = new URLSearchParams(paramsString);
    let keys = "";
    for (const key of searchParams.keys()) {
      keys = keys + key;
    }
    expect(keys).toEqual("key1key2");
  });

  it("should get value parameters in the query string", () => {
    const paramsString = "key1=value1&key2=value2";
    const searchParams = new URLSearchParams(paramsString);
    let values = "";
    for (const value of searchParams.values()) {
      values = values + value;
    }
    expect(values).toEqual("value1value2");
  });
});

describe("Blob class", () => {
  it("should construct a new Blob object with the provided data and options", () => {
    const blobData = ["Hello, world!"];
    const blobOptions = { type: "text/plain" };
    const blob = new Blob(blobData, blobOptions);

    expect(blob.size).toEqual(blobData[0].length);
    expect(blob.type).toEqual(blobOptions.type);
  });

  it("should create a Blob with default type if options.type is not provided", () => {
    const blobData = ["Hello, world!"];
    const blob = new Blob(blobData);

    expect(blob.size).toEqual(blobData[0].length);
    expect(blob.type).toEqual("");
  });

  it("should create a Blob with an empty array if no data is provided", () => {
    // @ts-ignore
    const blob = new Blob();

    expect(blob.size).toEqual(0);
    expect(blob.type).toEqual("");
  });

  it("should handle line endings properly", async () => {
    const text = "This\r\n is a \ntest\r\n string";

    // @ts-ignore
    const blob = new Blob([text], {
      // @ts-ignore
      endings: "native",
    });

    expect(blob.type).toEqual("");
    if (process.platform != "win32") {
      expect(blob.size < text.length).toBeTruthy();
      expect(await blob.text()).toEqual(text.replace(/\r\n/g, "\n"));
    }
  });

  it("should return an ArrayBuffer with the arrayBuffer() method", async () => {
    const blobData = ["Hello, world!"];
    const blob = new Blob(blobData, { type: "text/plain" });

    const arrayBuffer = await blob.arrayBuffer();

    expect(arrayBuffer).toBeInstanceOf(ArrayBuffer);
  });

  it("should return an Uint8Array with the bytes() method", async () => {
    const blobData = ["Hello, world!"];
    const blob = new Blob(blobData, { type: "text/plain" });

    const bytes = await blob.bytes();

    expect(bytes).toBeInstanceOf(Uint8Array);
  });

  it("should return a DataView with the slice method", () => {
    const blobData = ["Hello, world!"];
    const blob = new Blob(blobData, { type: "text/plain" });

    const slicedBlob = blob.slice(0, 5, "text/plain");

    expect(slicedBlob instanceof Blob).toBeTruthy();
    expect(slicedBlob.size).toEqual(5);
    expect(slicedBlob.type).toEqual("text/plain");
  });
});

describe("URL Utility Functions", () => {
  it("converts URL object to http options with urlToHttpOptions", () => {
    const url = new URL(
      "https://user:password@example.com:8080/path/to/file?param1=value1&param2=value2#fragment"
    );
    const options = urlModule.urlToHttpOptions(url);

    expect(options).toEqual({
      protocol: "https:",
      hostname: "example.com",
      hash: "fragment",
      search: "?param1=value1&param2=value2",
      pathname: "/path/to/file",
      path: "/path/to/file?param1=value1&param2=value2",
      href: "https://user:password@example.com:8080/path/to/file?param1=value1&param2=value2#fragment",
      auth: "user:password",
      port: "8080",
    });
  });

  it("handles URL without credentials or port with urlToHttpOptions", () => {
    const url = new URL("http://example.com/path/to/file");
    const options = urlModule.urlToHttpOptions(url);

    expect(options).toEqual({
      protocol: "http:",
      hostname: "example.com",
      pathname: "/path/to/file",
      path: "/path/to/file",
      href: "http://example.com/path/to/file",
    });
  });

  it("converts punycode domain to unicode with domainToUnicode", () => {
    const unicodeDomain = urlModule.domainToUnicode("xn--d1mi3b5c.com");
    expect(unicodeDomain).toBe("㶠㶤㷀㶱.com");
  });

  it("handles already unicode domain with domainToUnicode", () => {
    const unicodeDomain = urlModule.domainToUnicode("example.com");
    expect(unicodeDomain).toBe("example.com");
  });

  it("converts unicode domain to punycode with domainToASCII", () => {
    const asciiDomain = urlModule.domainToASCII("example.com");
    expect(asciiDomain).toBe("example.com"); // No conversion needed
  });

  it("converts non-ASCII domain to punycode with domainToASCII", () => {
    const asciiDomain = urlModule.domainToASCII("مثال.com");

    expect(asciiDomain).toBe("xn--mgbh0fb.com");
  });

  it("converts file URL to system path with fileURLToPath", () => {
    const url = new URL("file:///path/to/file.txt");
    const path = urlModule.fileURLToPath(url);

    expect(path).toBe("/path/to/file.txt"); // Platform specific path handling might differ
  });

  it("converts system path to file URL with pathToFileURL", () => {
    if (IS_WINDOWS) {
      const url = urlModule.pathToFileURL("C:/path/to/file.txt");
      expect(url.href).toBe("file:///C:/path/to/file.txt");
    } else {
      const url = urlModule.pathToFileURL("/path/to/file.txt");
      expect(url.href).toBe("file:///path/to/file.txt"); // Platform specific path handling might differ
    }
  });

  it("formats URL object into a string with format", () => {
    const url = new URL("https://a:b@測試?abc#foo");

    expect(url.href).toBe("https://a:b@xn--g6w251d/?abc#foo");

    expect(
      urlModule.format(url, { fragment: false, unicode: true, auth: false })
    ).toBe("https://測試/?abc");
  });
});

describe("File class", () => {
  it("should construct a new File", () => {
    const file = new File(["Hello, world!"], "hello.txt", {
      type: "text/plain",
    });

    expect(file.size).toBe(13);
    expect(file.type).toBe("text/plain");
    expect(file.name).toBe("hello.txt");
  });

  it("should return the correct lastModified date", () => {
    const fileWithDate = new File([], "file.bin", {
      lastModified: new Date(Date.UTC(2017, 1, 1, 0, 0, 0, 0)).getTime(),
    });
    expect(fileWithDate.lastModified).toBe(1485907200000);
  });

  it("has a name", () => {
    const file = new File(["file content"], "example.txt");
    expect(file.name).toBe("example.txt");
  });

  it("has content", () => {
    const file = new File(["file content"], "example.txt");
    expect(file.size).toBeGreaterThan(0);
  });

  it("has a size", () => {
    const file = new File(["file content"], "example.txt");
    expect(file.size).toBeGreaterThan(0);
  });

  it("has a type", () => {
    const file = new File(["file content"], "example.txt", {
      type: "text/plain",
    });
    expect(file.type).toBe("text/plain");
  });

  it("has last modified date", () => {
    const file = new File(["file content"], "example.txt");
    const now = new Date();
    expect(file.lastModified * 0.9999).toBeLessThanOrEqual(now.getTime());
  });

  it("can slice file", () => {
    const file = new File(["file content"], "example.txt");
    const slice = file.slice(0, 5);
    expect(slice).toBeInstanceOf(Blob);
    expect(slice.size).toBe(5);
  });

  it("can read file as text", async () => {
    const file = new File(["file content"], "example.txt");
    const text = await file.text();
    expect(text).toBe("file content");
  });

  it("can read file as arrayBuffer", async () => {
    const file = new File([1, 2, 3, 4] as any, "example.txt");
    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);
    expect(Array.from(uint8Array)).toStrictEqual([49, 50, 51, 52]);
    expect(uint8Array.length).toBe(4);
  });

  it("is an instance of Blob", () => {
    const file = new File(["file content"], "example.txt");
    expect(file).toBeInstanceOf(Blob);
  });
});
