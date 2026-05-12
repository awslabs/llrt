import { makeRunner, loadMetaScripts } from "./_harness-util.js";

const LOCATION = {
  href: "http://web-platform.test:8000/fetch/api/resources/",
  origin: "http://web-platform.test:8000",
  protocol: "http:",
  host: "web-platform.test:8000",
  hostname: "web-platform.test",
  port: "8000",
  pathname: "/fetch/api/resources/",
  search: "",
  hash: "",
  toString() {
    return this.href;
  },
};

const TEMPLATES = [
  [/\{\{host\}\}/g, "web-platform.test"],
  [/\{\{domains\[www2\]\}\}/g, "www2.web-platform.test"],
  [/\{\{ports\[http\]\[0\]\}\}/g, "8000"],
  [/\{\{ports\[http\]\[1\]\}\}/g, "8001"],
  [/\{\{ports\[https\]\[0\]\}\}/g, "8443"],
  [/\{\{ports\[https\]\[1\]\}\}/g, "8444"],
];
const substitute = (s) =>
  TEMPLATES.reduce((acc, [re, v]) => acc.replace(re, v), s);

function makeFetch() {
  const upstream = globalThis.fetch;
  return (url, option) => {
    // `url` may be a Request or URL object — pass it through unchanged and
    // only rewrite when it's an actual string path (the harness's relative /
    // bare-name affordance).
    if (typeof url !== "string") {
      return upstream(url, option);
    }
    let absolute = url;
    if (url.startsWith("../") || url.startsWith("./")) {
      absolute = new URL(url, LOCATION.href).href;
    } else if (url.startsWith("/")) {
      absolute = new URL(url, LOCATION.origin).href;
    } else if (
      !url.includes("://") &&
      !url.startsWith("data:") &&
      !url.startsWith("about:") &&
      !url.startsWith("blob:")
    ) {
      absolute = LOCATION.href + url;
    }
    return upstream(absolute, option);
  };
}

export const runTestDynamic = makeRunner({
  context: () => ({
    extras: {
      token: () => crypto.randomUUID(),
      location: LOCATION,
      RESOURCES_DIR: "../resources/",
      fetch: makeFetch(),
    },
    scripts: [
      "encoding/resources/encodings.js",
      "common/get-host-info.sub.js",
      "fetch/api/resources/keepalive-helper.js",
      "fetch/api/resources/utils.js",
      "fetch/api/request/request-cache.js",
      "fetch/api/request/request-error.js",
    ],
  }),
  postSetup(context) {
    // get-host-info.sub.js keeps `{{…}}` placeholders because the WPT
    // template server isn't in the loop for us; resolve them here.
    const origFn = context.get_host_info;
    if (origFn) {
      context.get_host_info = () => {
        const info = origFn();
        for (const k of Object.keys(info)) {
          if (typeof info[k] === "string") info[k] = substitute(info[k]);
        }
        return info;
      };
    }
  },
  wrap(source, { testDir }) {
    const substituted = substitute(source);
    return [substituted, loadMetaScripts(substituted, testDir)];
  },
});
