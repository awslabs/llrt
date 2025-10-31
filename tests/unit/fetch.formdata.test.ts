describe("FormData class", () => {
  it("should append and get string values", () => {
    const fd = new FormData();

    fd.append("name", "Alice");
    fd.append("age", "30");

    expect(fd.get("name")).toEqual("Alice");
    expect(fd.get("age")).toEqual("30");
  });

  it("should store Blob and File values correctly", () => {
    const fd = new FormData();

    const blob = new Blob(["hello"], { type: "text/plain" });
    const file = new File(["123"], "test.txt", { type: "text/plain" });

    fd.append("blob", blob);
    fd.append("file", file);

    expect(fd.get("blob")).toBeInstanceOf(Blob);
    expect(fd.get("file")).toBeInstanceOf(File);
    expect(fd.get("file").name).toEqual("test.txt");
  });

  it("should overwrite existing keys when set() is used", () => {
    const fd = new FormData();

    fd.append("color", "red");
    fd.set("color", "blue");

    expect(fd.get("color")).toEqual("blue");
  });

  it("should delete an entry when delete() is called", () => {
    const fd = new FormData();
    fd.append("token", "abc123");
    expect(fd.has("token")).toBeTruthy();

    fd.delete("token");
    expect(fd.has("token")).toBeFalsy();
  });

  it("should correctly report if a key exists using has()", () => {
    const fd = new FormData();
    fd.append("flag", "true");

    expect(fd.has("flag")).toBeTruthy();
    expect(fd.has("missing")).toBeFalsy();
  });

  it("should return all keys with keys()", () => {
    const fd = new FormData();
    fd.append("a", "1");
    fd.append("b", "2");

    const keys = fd.keys();
    expect(keys).toContain("a");
    expect(keys).toContain("b");
  });

  it("should return all values with values()", () => {
    const fd = new FormData();
    fd.append("a", "apple");
    fd.append("b", "banana");

    const values = fd.values();
    expect(values).toContain("apple");
    expect(values).toContain("banana");
  });

  it("should iterate entries() properly", () => {
    const fd = new FormData();
    fd.append("x", "100");
    fd.append("y", "200");

    const collected = Array.from(fd.entries());
    expect(collected).toEqual([
      ["x", "100"],
      ["y", "200"],
    ]);
  });

  it("should call forEach() for all entries in order", () => {
    const fd = new FormData();
    fd.append("a", "1");
    fd.append("b", "2");

    const result = [];
    fd.forEach((value, key) => {
      result.push([key, value]);
    });

    expect(result).toEqual([
      ["a", "1"],
      ["b", "2"],
    ]);
  });

  it("should handle multiple values for the same key using append()", () => {
    const fd = new FormData();
    fd.append("tag", "news");
    fd.append("tag", "tech");

    const all = fd.getAll("tag");
    expect(all).toEqual(["news", "tech"]);
  });
});
