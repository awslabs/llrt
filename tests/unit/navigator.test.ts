describe("navigator.userAgent", () => {
  it('should start with "llrt "', () => {
    expect(navigator.userAgent.startsWith("llrt ")).toBeTruthy();
  });
});
