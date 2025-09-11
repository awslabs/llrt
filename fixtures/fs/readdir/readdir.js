import fs from "node:fs/promises";

fs.readdir("./", { recursive: true }).then((res) => {});
