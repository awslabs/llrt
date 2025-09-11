import type dns from "node:dns";

import defaultImport from "node:dns";
import legacyImport from "dns";

it("node:dns should be the same as dns", () => {
  expect(defaultImport).toStrictEqual(legacyImport);
});

const { lookup } = defaultImport;

// Promise wrapper for dns.lookup
const dnsLookupAsync = (
  hostname: string,
  options?: number | dns.LookupOptions
) =>
  new Promise<dns.LookupAddress>((resolve, reject) => {
    lookup(hostname, options as any, (err, address, family) => {
      if (err) reject(err);
      else resolve({ address, family });
    });
  });

describe("lookup", () => {
  it("localhost name resolution should be possible (optionless)", async () => {
    const { address, family } = await dnsLookupAsync("localhost");
    expect(address === "::1" || address === "127.0.0.1").toBeTruthy();
    expect(family === 4 || family === 6).toBeTruthy();
  });

  it("localhost name resolution should be possible (integer option)", async () => {
    const { address, family } = await dnsLookupAsync("localhost", 4);
    expect(address).toEqual("127.0.0.1");
    expect(family).toEqual(4);
  });

  it("localhost name resolution should be possible (record option)", async () => {
    const { address, family } = await dnsLookupAsync("localhost", {
      family: 4,
    });
    expect(address).toEqual("127.0.0.1");
    expect(family).toEqual(4);
  });

  if (process.platform !== "linux") {
    it("Name resolution for localhost2 should result in an error (integer option)", async () => {
      await expect(dnsLookupAsync("localhost2", 4)).rejects.toThrow("known");
    });

    it("Name resolution for localhost2 should result in an error (optionless)", async () => {
      await expect(dnsLookupAsync("localhost2")).rejects.toThrow("known");
    });

    it("Name resolution for localhost2 should result in an error (record option)", async () => {
      await expect(dnsLookupAsync("localhost2", { family: 4 })).rejects.toThrow(
        "known"
      );
    });
  }
});
