const dns = require("dns");

describe("lookup", () => {
  it("optionless", (done) => {
    dns.lookup("localhost", (err, address, family) => {
      expect(address === "::1" || address === "127.0.0.1").toBeTruthy();
      expect(family === 4 || family === 6).toBeTruthy();
      done();
    });
  });

  it("option - integer", (done) => {
    dns.lookup("localhost", 4, (err, address, family) => {
      expect(address).toEqual("127.0.0.1");
      expect(family).toEqual(4);
      done();
    });
  });

  it("option - record", (done) => {
    dns.lookup("localhost", { family: 4 }, (err, address, family) => {
      expect(address).toEqual("127.0.0.1");
      expect(family).toEqual(4);
      done();
    });
  });
});
