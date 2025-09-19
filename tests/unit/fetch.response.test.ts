describe("Response.json()", () => {
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
});
