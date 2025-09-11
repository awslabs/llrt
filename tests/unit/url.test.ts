import defaultImport from "node:url";
import legacyImport from "url";

import { platform } from "node:os";
const IS_WINDOWS = platform() === "win32";

it("node:url should be the same as url", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const {
  format,
  URL,
  URLSearchParams,
  urlToHttpOptions,
  domainToUnicode,
  domainToASCII,
  fileURLToPath,
  pathToFileURL,
} = defaultImport;

describe("URL module import", () => {
  it("global URL and imported URL are equal", () => {
    const testUrl = "https://www.example.com";
    const moduleUrl = new URL(testUrl);
    const globalUrl = new globalThis.URL(testUrl);
    expect(moduleUrl).toEqual(globalUrl);
  });
  it("global URLSearchParams and imported URLSearchParams are equal", () => {
    const paramsString = "topic=api&a=1&a=2&a=3";
    const moduleSearchParams = new URLSearchParams(paramsString);
    const globalSearchParams = new globalThis.URLSearchParams(paramsString);
    expect(moduleSearchParams).toEqual(globalSearchParams);
  });
  describe("import { URL } from 'url';", () => {
    it("should parse a url hostname", () => {
      const testUrl = new URL("https://www.example.com");
      expect(testUrl.protocol).toEqual("https:");
      expect(testUrl.host).toEqual("www.example.com");
      expect(testUrl.hostname).toEqual("www.example.com");
    });
    it("toString method works", () => {
      const testUrl = new URL("/base", "https://www.example.com");
      expect(testUrl.toString()).toEqual("https://www.example.com/base");
    });
    it("canParse method works", () => {
      const validCanParse = URL.canParse("https://www.example.com");
      const invalidCanParse = URL.canParse("not_valid");
      expect(validCanParse).toEqual(true);
      expect(invalidCanParse).toEqual(false);
      expect(URL.canParse("/foo", "https://example.org/")).toEqual(true);
    });
  });

  describe("import { URLSearchParams } from 'url';", () => {
    it("supports URLSearchParams basic API", () => {
      const paramsString = "topic=api&a=1&a=2&a=3";
      const searchParams = new URLSearchParams(paramsString);
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

describe("URL Utility Functions", () => {
  it("converts URL object to http options with urlToHttpOptions", () => {
    const url = new URL(
      "https://user:password@example.com:8080/path/to/file?param1=value1&param2=value2#fragment"
    );
    const options = urlToHttpOptions(url);

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
    const options = urlToHttpOptions(url);

    expect(options).toEqual({
      protocol: "http:",
      hostname: "example.com",
      pathname: "/path/to/file",
      path: "/path/to/file",
      href: "http://example.com/path/to/file",
    });
  });

  it("converts punycode domain to unicode with domainToUnicode", () => {
    const unicodeDomain = domainToUnicode("xn--d1mi3b5c.com");
    expect(unicodeDomain).toBe("㶠㶤㷀㶱.com");
  });

  it("handles already unicode domain with domainToUnicode", () => {
    const unicodeDomain = domainToUnicode("example.com");
    expect(unicodeDomain).toBe("example.com");
  });

  it("converts unicode domain to punycode with domainToASCII", () => {
    const asciiDomain = domainToASCII("example.com");
    expect(asciiDomain).toBe("example.com"); // No conversion needed
  });

  it("converts non-ASCII domain to punycode with domainToASCII", () => {
    const asciiDomain = domainToASCII("مثال.com");

    expect(asciiDomain).toBe("xn--mgbh0fb.com");
  });

  it("converts file URL to system path with fileURLToPath", () => {
    const url = new URL("file:///path/to/file.txt");
    const path = fileURLToPath(url);

    expect(path).toBe("/path/to/file.txt"); // Platform specific path handling might differ
  });

  it("converts system path to file URL with pathToFileURL", () => {
    if (IS_WINDOWS) {
      const url = pathToFileURL("C:/path/to/file.txt");
      expect(url.href).toBe("file:///C:/path/to/file.txt");
    } else {
      const url = pathToFileURL("/path/to/file.txt");
      expect(url.href).toBe("file:///path/to/file.txt"); // Platform specific path handling might differ
    }
  });

  it("formats URL object into a string with format", () => {
    const url = new URL("https://a:b@測試?abc#foo");

    expect(url.href).toBe("https://a:b@xn--g6w251d/?abc#foo");

    expect(format(url, { fragment: false, unicode: true, auth: false })).toBe(
      "https://測試/?abc"
    );
  });
});
