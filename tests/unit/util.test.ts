const util = require("util");
const EventEmitter = require("events");

describe("Util.inherits", () => {
  it("should be inheritable parent classes", () => {
    function MyStream() {
      EventEmitter.call(this);
    }

    util.inherits(MyStream, EventEmitter);

    const stream = new MyStream();

    expect(stream instanceof EventEmitter).toBeTruthy();
    expect(MyStream.super_).toEqual(EventEmitter);
  });
});
