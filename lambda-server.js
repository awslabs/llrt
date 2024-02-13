import pureHttp from "pure-http";
import fs from "fs";

const http = require('http');
const https = require('https');
const PORT = 3000;
const BASE_PATH = "/2018-06-01/runtime";
const ARGS = process.argv.slice(2);

let httpMode = false;
let eventJson = null;

let argIndex = 0;
for (let arg of ARGS) {
  if (arg == "-h" || arg == "--http") {
    httpMode = true;
  }

  if (arg == "-e" || arg == "--event") {
    eventJson = JSON.parse(fs.readFileSync(ARGS[argIndex + 1]).toString());
  }
  argIndex++;
}

const app = pureHttp();

const pendingRequests = [];
const requests = {};
let invocationWaiter = null;

app.use(async (req, res, next) => {
  const body = await new Promise((resolve, reject) => {
    let buffer = "";
    req
      .on("data", (chunk) => {
        buffer += chunk;
      })
      .on("end", () => {
        resolve(buffer);
      })
      .on("error", reject);
  });
  if (body) {
    req.body = body || undefined;
  }
  return next();
});

app.use((req, res, next) => {
  const date = new Date();
  const hours = date.getHours();
  const minutes = date.getMinutes();
  const seconds = date.getSeconds();
  const milliseconds = date.getMilliseconds();
  console.log(
    `${hours}:${minutes}:${seconds}:${milliseconds} - ${req.method}: ${req.path}`
  );
  try {
    return next();
  } catch (error) {
    res.json({ error });
  }
});

app.use((req, res, next) => {
  res.setHeader("Access-Control-Allow-Origin", "*");
  res.setHeader("Access-Control-Allow-Headers", "*");
  res.setHeader("Access-Control-Allow-Methods", "*");
  if (req.method === "OPTIONS") {
    return res.send("");
  }
  next();
});

app.get(`${BASE_PATH}/invocation/next`, async (req, res) => {
  res.header("lambda-runtime-deadline-ms", Date.now() + 1000 * 60);
  if (httpMode) {
    if (pendingRequests.length == 0) {
      await new Promise((resolve) => {
        invocationWaiter = resolve;
      });
    }
    let { id, event } = pendingRequests.shift();

    res.header("lambda-runtime-aws-request-id", id);
    res.json(event);
  } else {
    res.header("lambda-runtime-aws-request-id", "1234");

    res.json(
      eventJson || {
        key1: "value1",
        key2: "value2",
        key3: "value3",
      }
    );
  }
});

app.post(`${BASE_PATH}/init/error`, (req, res) => {
  if (httpMode) {
    for (const request of pendingRequests) {
      request.reject({
        statusCode: 500,
        body: req.body,
        headers: { "content-type": "application/json" },
      });
    }
  }

  res.status(202);
  res.send();
});

app.post(`${BASE_PATH}/invocation/:id/response`, (req, res) => {
  if (httpMode) {
    const { id } = req.params;

    const { resolve } = requests[id];
    delete requests[id];

    resolve(req.body);
  } else {
    console.log(req.body);
  }

  res.status(202);
  res.send();
});

app.post(`${BASE_PATH}/invocation/:id/error`, (req, res) => {
  if (httpMode) {
    const { id } = req.params;

    const { reject } = requests[id];
    delete requests[id];

    reject({
      body: req.body,
      statusCode: 500,
      headers: { "content-type": "application/json" },
    });
  } else {
    console.error(req.body);
  }

  res.status(202);
  res.send();
});

app.all("*", async (req, res) => {
  if (!httpMode) {
    res.status(400);
    res.send("Server is not in HTTP mode. Start with -h flag");
    return;
  }

  const requestUrl = new URL(`http://localhost:0000${req.url}`);

  const rawQueryString = requestUrl.search?.substring(1);

  const requestId = Math.random().toString(36).substring(2);

  const queryStringParameters = Object.fromEntries(
    requestUrl.searchParams.entries()
  );

  const event = {
    version: "2.0",
    routeKey: "$default",
    rawPath: "/my/path",
    rawQueryString,
    cookies: undefined,
    headers: Object.entries(req.headers).reduce((acc, [key, value]) => {
      acc[key] = (value && Array.isArray(value) && value.join(",")) || value;
      return acc;
    }, {}),
    queryStringParameters,
    requestContext: {
      accountId: "123456789012",
      apiId: "localhost",
      domainName: "localhost",
      domainPrefix: "localhost",
      http: {
        method: req.method,
        path: req.path,
        protocol: req.protocol,
        sourceIp: "192.168.1.1",
        userAgent: req.header["User-Agent"],
      },
      requestId,
      routeKey: "$default",
      stage: "$default",
      time: new Date().toString(),
      timeEpoch: new Date().getTime(),
    },
    body: req.body,
    pathParameters: req.params,
    isBase64Encoded: false,
  };

  const responsePromise = new Promise((resolve, reject) => {
    const id = requestId;
    const request = {
      event,
      resolve,
      reject,
      id,
    };
    pendingRequests.push(request);
    requests[id] = request;
  });

  if (invocationWaiter) {
    invocationWaiter();
  }

  let result;
  try {
    result = await responsePromise;
  } catch (e) {
    result = e;
  }

  try {
    result = JSON.parse(result);
  } catch (_) {}

  if (result.body && result.statusCode) {
    if (result.headers) {
      for (const key in result.headers) {
        res.setHeader(key, result.headers[key]);
      }
    }
    res.status(result.statusCode);
    if (result.isBase64Encoded) {
      res.send(Buffer.from(result.body, "base64"));
    } else {
      res.send(result.body);
    }
    return;
  }

  res.send(result);
});

app.listen(PORT, () => {
  console.log(`Server started on port ${PORT}`);
  if (httpMode) {
    console.log(`- HTTP: http://localhost:${PORT}`);
  }
});
