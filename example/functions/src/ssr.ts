import fs from "fs/promises";
import App from "./react/App";
import ReactDOMServer from "react-dom/server";

import React from "react";
import API from "./api";

type Method = "GET" | "POST" | "PUT" | "DELETE";

type ResponseOptions = {
  contentType?: string;
  isBase64Encoded?: boolean;
  headers?: Record<string, string>;
  statusCode?: number;
};

const ASSET_CACHE: Record<string, string> = {};
const MIME_TYPES = {
  js: "text/javascript",
  css: "text/css",
  html: "text/html",
  ico: "image/x-icon",
  svg: "image/svg+xml",
  png: "image/png",
  jpg: "image/jpeg",
};

const htmlFilePromise = fs.readFile("./index.html");

let htmlContent: string | null = null;

class HttpError extends Error {
  status: number;
  constructor(status: number, message: string) {
    super(message);
    this.status = status;
  }
}

const response = (
  body: string,
  {
    contentType = "text/plain",
    isBase64Encoded,
    headers,
    statusCode = 200,
  }: ResponseOptions = {}
) => ({
  statusCode,
  headers: {
    "content-type": contentType,
    ...headers,
  },
  body,
  isBase64Encoded,
});

const apiResponse = async (
  pathParams: string[],
  method: Method,
  body?: string
) => {
  const [id] = pathParams;
  if (id) {
    if (method === "DELETE") {
      await API.delete(id);
      return response("", {
        contentType: "application/json",
      });
    }
    if (method === "PUT") {
      const { text, completedDate } = JSON.parse(body);
      const item = await API.update({ id, text, completedDate });
      return response(JSON.stringify(item), {
        contentType: "application/json",
      });
    }
  }

  if (pathParams.length == 0 && method === "POST") {
    const { text } = JSON.parse(body);
    const item = await API.create(text);
    return response(JSON.stringify(item), {
      contentType: "application/json",
    });
  }

  throw new HttpError(404, "Not found");
};

const appResponse = async () => {
  let todoItems;
  if (!htmlContent) {
    const [html, items] = await Promise.all([htmlFilePromise, API.getAll()]);
    htmlContent = html.toString();
    todoItems = items;
  } else {
    todoItems = await API.getAll();
  }

  const app = ReactDOMServer.renderToString(
    React.createElement(App, { todoItems })
  );
  const html = htmlContent
    .replace(
      '<script id="init" type="text/javascript"></script>',
      `<script id="init" type="text/javascript">
window.todoItems = ${JSON.stringify(todoItems)};
window.releaseName = ${JSON.stringify(process.release.name)}
</script>`
    )
    .replace('<div id="root"></div>', `<div id="root">${app}</div>`);

  return response(html, { contentType: "text/html" });
};

const fileExists = (file: string) =>
  fs.access(file).then(
    () => true,
    () => false
  );

const loadAsset = async (asset: string) => {
  const safeAsset = asset.replace("..", "");
  const cachedAsset = ASSET_CACHE[safeAsset];
  if (cachedAsset) {
    return cachedAsset;
  }
  if (!(await fileExists(safeAsset))) {
    throw new HttpError(404, "Not found");
  }
  const data = (await fs.readFile(safeAsset)).toString("base64");
  ASSET_CACHE[safeAsset] = data;
  return data;
};

const assetResponse = async (path: string) => {
  const data = await loadAsset(path);
  const extIndex = path.lastIndexOf(".");
  let contentType = null;
  if (extIndex > -1) {
    const ext = path.substring(extIndex + 1);
    contentType = MIME_TYPES[ext as keyof typeof MIME_TYPES];
  }

  return response(data, { contentType, isBase64Encoded: true });
};

export const handler = async (event: any) => {
  const { method = "GET", path: eventPath = "/" } =
    event?.requestContext?.http || {};

  try {
    const reqSegments: string[] = (eventPath as string)
      .split("/")
      .filter((x) => x)
      .slice(1);
    console.log({ reqSegments, eventPath, method });

    if (reqSegments[0] === "api") {
      return await apiResponse(reqSegments.slice(1), method, event.body);
    }

    if (method === "GET") {
      if (reqSegments.length === 0) {
        return await appResponse();
      }
      return await assetResponse(reqSegments.join("/"));
    }
    throw new HttpError(400, "Method not supported");
  } catch (e) {
    console.error(e);
    if (e instanceof HttpError) {
      return {
        statusCode: e.status,
        body: e.message,
      };
    }
    return {
      statusCode: 500,
      body: "Internal server error",
    };
  }
};
