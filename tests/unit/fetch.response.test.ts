describe("Response class", () => {
  it("should construct a new Response object with default values", () => {
    const response = new Response();
    expect(response.status).toEqual(200);
    expect(response.statusText).toEqual("OK");
    expect(response.headers instanceof Headers).toBeTruthy();
    // Per WHATWG Fetch spec, body is null for empty response
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
    // Per WHATWG Fetch spec, body is null for error response
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
});
