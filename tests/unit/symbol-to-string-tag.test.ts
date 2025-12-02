describe("Symbol.toStringTag", () => {
  describe("URL module", () => {
    it("URL should have correct Symbol.toStringTag", () => {
      const url = new URL("https://example.com");
      expect(url[Symbol.toStringTag]).toBe("URL");
      expect(Object.prototype.toString.call(url)).toBe("[object URL]");
    });

    it("URLSearchParams should have correct Symbol.toStringTag", () => {
      const params = new URLSearchParams("foo=bar");
      expect(params[Symbol.toStringTag]).toBe("URLSearchParams");
      expect(Object.prototype.toString.call(params)).toBe(
        "[object URLSearchParams]"
      );
    });
  });

  describe("Encoding", () => {
    it("TextEncoder should have correct Symbol.toStringTag", () => {
      const encoder = new TextEncoder();
      expect(encoder[Symbol.toStringTag]).toBe("TextEncoder");
      expect(Object.prototype.toString.call(encoder)).toBe(
        "[object TextEncoder]"
      );
    });

    it("TextDecoder should have correct Symbol.toStringTag", () => {
      const decoder = new TextDecoder();
      expect(decoder[Symbol.toStringTag]).toBe("TextDecoder");
      expect(Object.prototype.toString.call(decoder)).toBe(
        "[object TextDecoder]"
      );
    });
  });

  describe("Abort API", () => {
    it("AbortController should have correct Symbol.toStringTag", () => {
      const controller = new AbortController();
      expect(controller[Symbol.toStringTag]).toBe("AbortController");
      expect(Object.prototype.toString.call(controller)).toBe(
        "[object AbortController]"
      );
    });

    it("AbortSignal should have correct Symbol.toStringTag", () => {
      const controller = new AbortController();
      const signal = controller.signal;
      expect(signal[Symbol.toStringTag]).toBe("AbortSignal");
      expect(Object.prototype.toString.call(signal)).toBe(
        "[object AbortSignal]"
      );
    });
  });

  describe("Fetch API", () => {
    it("Headers should have correct Symbol.toStringTag", () => {
      const headers = new Headers();
      expect(headers[Symbol.toStringTag]).toBe("Headers");
      expect(Object.prototype.toString.call(headers)).toBe("[object Headers]");
    });

    it("Request should have correct Symbol.toStringTag", () => {
      const request = new Request("https://example.com");
      expect(request[Symbol.toStringTag]).toBe("Request");
      expect(Object.prototype.toString.call(request)).toBe("[object Request]");
    });

    it("Response should have correct Symbol.toStringTag", () => {
      const response = new Response();
      expect(response[Symbol.toStringTag]).toBe("Response");
      expect(Object.prototype.toString.call(response)).toBe(
        "[object Response]"
      );
    });

    it("FormData should have correct Symbol.toStringTag", () => {
      const formData = new FormData();
      expect(formData[Symbol.toStringTag]).toBe("FormData");
      expect(Object.prototype.toString.call(formData)).toBe(
        "[object FormData]"
      );
    });
  });

  describe("Blob API", () => {
    it("Blob should have correct Symbol.toStringTag", () => {
      const blob = new Blob(["test"]);
      expect(blob[Symbol.toStringTag]).toBe("Blob");
      expect(Object.prototype.toString.call(blob)).toBe("[object Blob]");
    });

    it("File should have correct Symbol.toStringTag", () => {
      const file = new File(["test"], "test.txt");
      expect(file[Symbol.toStringTag]).toBe("File");
      expect(Object.prototype.toString.call(file)).toBe("[object File]");
    });
  });

  describe("Crypto API", () => {
    it("CryptoKey should have correct Symbol.toStringTag", async () => {
      const key = await crypto.subtle.generateKey(
        { name: "HMAC", hash: "SHA-256" },
        false,
        ["sign", "verify"]
      );
      expect(key[Symbol.toStringTag]).toBe("CryptoKey");
      expect(Object.prototype.toString.call(key)).toBe("[object CryptoKey]");
    });
  });

  describe("Exceptions", () => {
    it("DOMException should have correct Symbol.toStringTag", () => {
      const exception = new DOMException("test", "TestError");
      expect(exception[Symbol.toStringTag]).toBe("DOMException");
      expect(Object.prototype.toString.call(exception)).toBe(
        "[object DOMException]"
      );
    });
  });
});
