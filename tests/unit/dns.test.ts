const dns = require("dns");

describe("lookup", () => {
  it("localhost name resolution should be possible (optionless)", (done) => {
    dns.lookup("localhost", (err, address, family) => {
      expect(err).toBeNull();
      expect(address === "::1" || address === "127.0.0.1").toBeTruthy();
      expect(family === 4 || family === 6).toBeTruthy();
      done();
    });
  });

  it("Name resolution for localhost2 should result in an error (optionless)", () => {
    dns.lookup("localhost2", (err, address, family) => {
      expect(err.message).toEqual(
        "failed to lookup address information: nodename nor servname provided, or not known"
      );
    });
  });

  it("localhost name resolution should be possible (integer option)", (done) => {
    dns.lookup("localhost", 4, (err, address, family) => {
      expect(err).toBeNull();
      expect(address).toEqual("127.0.0.1");
      expect(family).toEqual(4);
      done();
    });
  });

  it("Name resolution for localhost2 should result in an error (integer option)", () => {
    dns.lookup("localhost2", 4, (err, address, family) => {
      expect(err.message).toEqual(
        "failed to lookup address information: nodename nor servname provided, or not known"
      );
    });
  });

  it("localhost name resolution should be possible (record option)", (done) => {
    dns.lookup("localhost", { family: 4 }, (err, address, family) => {
      expect(err).toBeNull();
      expect(address).toEqual("127.0.0.1");
      expect(family).toEqual(4);
      done();
    });
  });

  it("Name resolution for localhost2 should result in an error (record option)", () => {
    dns.lookup("localhost2", { family: 4 }, (err, address, family) => {
      expect(err.message).toEqual(
        "failed to lookup address information: nodename nor servname provided, or not known"
      );
    });
  });
});
