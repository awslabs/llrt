const events = [];
Promise.resolve().then(() => events.push("microtask"));
setTimeout(() => events.push("timer"), 0);
const start = Date.now();
while (Date.now() - start < 5);
require("node:stream");
events.push("required");
setTimeout(() => console.log(events.join(",")), 10);
