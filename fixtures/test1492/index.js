import { get, set } from "./lib/shared/store.js";
import { get_in_bar } from "./lib/bar/index.js";

set("foo", "bar");
expect(get("foo")).toBe("bar");
expect(get_in_bar("foo")).toBe("bar");
