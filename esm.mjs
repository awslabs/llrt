import path from "path";
import fs from "fs/promises";

const BUILTINS = new Set(
  Object.keys(process.binding("natives")).reduce((acc, key) => {
    acc.push(key);
    acc.push(`node:${key}`);
    return acc;
  }, [])
);

const CWD = process.cwd();

const loadJson = async (file) => JSON.parse(await fs.readFile(file));

const urlToPath = (urlString) => {
  const url = new URL(urlString).pathname;

  if (process.platform == "win32" && url.startsWith("/")) {
    return url.substring(1);
  }

  return url;
};

const fileExists = async (filename) => {
  return fs
    .stat(filename)
    .then((stat) => stat.isFile())
    .catch(() => false);
};

const resolution = (path, { format = "module", ...opts } = {}) => {
  if (!path.startsWith("/")) {
    path = `/${path}`;
  }
  path = path.replace(/\\/g, "/");
  return {
    url: `file://${path}`,
    format,
    shortCircuit: true,
    ...opts,
  };
};

const tryResolve = async (filepath, format = "module") => {
  if (await fileExists(filepath)) {
    if (filepath.endsWith(".json")) {
      const source = await loadJson(filepath);
      return resolution(filepath, {
        format: "commonjs",
        source: `module.exports = ${JSON.stringify(source, null, 2)}\n`,
        shortCircuit: true,
      });
    }
    return resolution(filepath, { format });
  }

  const parsedPath = path.parse(filepath);
  if (parsedPath.ext != ".js") {
    const jsPath = `${filepath}.js`;
    if (await fileExists(jsPath)) {
      return resolution(jsPath, { format });
    }
  }
  const directoryPath = path.join(filepath, "index.js");
  if (await fileExists(directoryPath)) {
    return resolution(directoryPath, { format });
  }
};

export async function resolve(specifier, context, nextLoad) {
  if (specifier.startsWith("file://")) {
    return {
      url: specifier,
      shortCircuit: true,
      format: "module",
    };
  }

  if (BUILTINS.has(specifier)) {
    return nextLoad(specifier, context);
  }

  let parentPath = context.parentURL
    ? path.dirname(urlToPath(context.parentURL))
    : CWD;

  const resolvedPath = path.join(parentPath, specifier);

  let res;
  if ((res = await tryResolve(resolvedPath))) {
    return res;
  }

  const modulePath = path.resolve(path.join("node_modules", specifier));
  if ((res = await tryResolve(modulePath))) {
    const filepath = urlToPath(res.url);

    const parsedPath = path.parse(filepath);
    const urlSegments = filepath.substring(parsedPath.root.length).split("/");
    while (urlSegments.length) {
      urlSegments.pop();
      const packageJsonPath = path.join(
        parsedPath.root,
        ...urlSegments,
        "package.json"
      );
      if (await fileExists(packageJsonPath)) {
        const packageJson = await loadJson(packageJsonPath);
        if (!packageJson.module) {
          res.format = packageJson.format;
        }

        break;
      }
    }
    return res;
  }

  const [firstSegment, secondSegment] = specifier.split(path.sep);
  let moduleDirectory = path.resolve(
    path.join("node_modules", firstSegment, secondSegment || "")
  );
  let modulePackageJsonPath = path.join(moduleDirectory, "package.json");
  if (await fileExists(modulePackageJsonPath)) {
    const pkg = await loadJson(modulePackageJsonPath);
    const moduleFile = path.join(moduleDirectory, pkg.module || pkg.main);
    if (await fileExists(moduleFile)) {
      return resolution(moduleFile, {
        format: pkg.module ? "module" : "commonjs",
      });
    }
  }

  return nextLoad(specifier, context);
}
