import process from "node:process";

import defaultImport from "node:process";
import legacyImport from "process";
import * as legacyNamedImport from "process";

const modules = {
  "node:process": defaultImport,
  process: legacyImport,
  "* as process": legacyNamedImport,
};
for (const module in modules) {
  const {
    env,
    cwd,
    argv0,
    argv,
    platform,
    arch,
    hrtime,
    release,
    version,
    versions,
    exit,
  } = modules[module];

  describe(module, () => {
    it("should have a process env", () => {
      expect(env).toEqual(process.env);
    });

    it("should have a process cwd", () => {
      expect(cwd()).toEqual(process.cwd());
    });

    it("should have a process argv0", () => {
      expect(argv0).toEqual(process.argv0);
    });

    it("should have a process argv", () => {
      expect(argv).toEqual(process.argv);
    });

    it("should have a process platform", () => {
      expect(platform).toEqual(process.platform);
    });

    it("should have a process arch", () => {
      expect(arch).toEqual(process.arch);
    });

    it("should have a process hrtime", () => {
      expect(hrtime.bigint() > 0).toBeTruthy();
    });

    it("should have a process release", () => {
      expect(release).toEqual(process.release);
    });

    it("should have a process version", () => {
      expect(version).toEqual(process.version);
    });

    it("should have a process versions", () => {
      expect(versions).toEqual(process.versions);
    });

    it("should have a process exit", () => {
      expect(exit).toEqual(process.exit);
    });
  });
}
