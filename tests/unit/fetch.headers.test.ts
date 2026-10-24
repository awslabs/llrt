import net from "node:net";

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

describe("Received headers", () => {
  it("combines repeated values while preserving cookies and cloned responses", async () => {
    const server = net.createServer((socket) => {
      let request = "";
      socket.on("data", (data) => {
        request += data.toString();
        if (!request.includes("\r\n\r\n")) return;
        socket.write(
          [
            "HTTP/1.1 200 OK",
            "Connection: close",
            "Content-Length: 2",
            "X-Single: untouched",
            "X-Values: first",
            "x-values:",
            "X-Values: last",
            "X-Empty:",
            "X-Empty:",
            "WWW-Authenticate: one, two",
            "WWW-Authenticate: three",
            "Set-Cookie: a=1; Expires=Wed, 21 Oct 2037 07:28:00 GMT",
            "Set-Cookie: b=2",
            "",
            "OK",
          ].join("\r\n")
        );
        socket.end();
      });
    });
    await new Promise<void>((resolve) =>
      server.listen(0, "127.0.0.1", resolve)
    );
    try {
      const address = server.address() as net.AddressInfo;
      const response = await fetch(`http://127.0.0.1:${address.port}/`);
      const clone = response.clone();
      const cookies = ["a=1; Expires=Wed, 21 Oct 2037 07:28:00 GMT", "b=2"];
      for (const current of [response, clone]) {
        const headers = current.headers;
        expect(headers.get("x-single")).toBe("untouched");
        expect(headers.get("X-Values")).toBe("first, , last");
        expect(headers.get("x-empty")).toBe(", ");
        expect(headers.get("www-authenticate")).toBe("one, two, three");
        expect(headers.getSetCookie()).toEqual(cookies);
        expect(headers.get("set-cookie")).toBe(cookies.join(", "));
        expect([...headers].filter(([name]) => name === "x-values")).toEqual([
          ["x-values", "first, , last"],
        ]);
        expect([...headers].filter(([name]) => name === "set-cookie")).toEqual(
          cookies.map((value) => ["set-cookie", value])
        );
        const values: string[] = [];
        headers.forEach((value, name) => {
          if (name === "x-values") values.push(value);
        });
        expect(values).toEqual(["first, , last"]);
        expect(new Headers(headers).get("x-values")).toBe("first, , last");
        expect(() => headers.append("x-values", "extra")).toThrow(TypeError);
        expect(() => headers.set("x-values", "replacement")).toThrow(TypeError);
        expect(() => headers.delete("x-values")).toThrow(TypeError);
      }
      expect(await response.text()).toBe("OK");
      expect(await clone.text()).toBe("OK");
    } finally {
      await new Promise<void>((resolve) => server.close(() => resolve()));
    }
  });
});
