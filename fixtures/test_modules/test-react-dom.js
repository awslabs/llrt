const assert = require("assert");

import * as dom1 from "elem-react-dom/server.edge";
assert.ok(dom1.name == "react-dom/server.edge");

const dom2 = require("elem-react-dom/server.edge");
assert.ok(dom2.name == "react-dom/server.edge");
