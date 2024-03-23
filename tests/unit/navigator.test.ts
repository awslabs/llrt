import defaultImport from "navigator";
import * as namedImport from "navigator";

describe("navigator.userAgent", () => {
  it("should have a navigator userAgent", () => {
    expect(defaultImport.userAgent).toEqual(navigator.userAgent);
    expect(namedImport.userAgent).toEqual(navigator.userAgent);
  });
  it("should start with \"llrt/\"", () => {
    expect(navigator.userAgent.startsWith("llrt/")).toBeTruthy();
  });
});
