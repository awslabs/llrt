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
