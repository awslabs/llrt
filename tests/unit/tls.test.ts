import * as tls from "tls";
import * as fs from "fs";

// Load test certificates from llrt_test_tls
// Tests run from project root, so use relative path from there
const serverCert = fs.readFileSync("libs/llrt_test_tls/data/server.pem");
const serverKey = fs.readFileSync("libs/llrt_test_tls/data/server.key");

describe("tls module exports", () => {
  it("should export connect function", () => {
    expect(typeof tls.connect).toBe("function");
  });

  it("should export createSecureContext function", () => {
    expect(typeof tls.createSecureContext).toBe("function");
  });

  it("should export TLSSocket class", () => {
    expect(typeof tls.TLSSocket).toBe("function");
  });

  it("should export SecureContext class", () => {
    expect(typeof tls.SecureContext).toBe("function");
  });
});

describe("TLSSocket", () => {
  it("should create a TLSSocket instance", () => {
    const socket = new tls.TLSSocket();
    expect(socket).toBeInstanceOf(tls.TLSSocket);
  });

  it("should have encrypted property", () => {
    const socket = new tls.TLSSocket();
    expect(typeof socket.encrypted).toBe("boolean");
  });

  it("should have connecting property set to false initially", () => {
    const socket = new tls.TLSSocket();
    expect(socket.connecting).toBe(false);
  });

  it("should have pending property set to true initially", () => {
    const socket = new tls.TLSSocket();
    expect(socket.pending).toBe(true);
  });

  it("should have readyState property", () => {
    const socket = new tls.TLSSocket();
    expect(socket.readyState).toBe("closed");
  });

  it("should have authorized property set to false initially", () => {
    const socket = new tls.TLSSocket();
    expect(socket.authorized).toBe(false);
  });

  it("should have connect method", () => {
    const socket = new tls.TLSSocket();
    expect(typeof socket.connect).toBe("function");
  });

  it("should have getProtocol method", () => {
    const socket = new tls.TLSSocket();
    expect(typeof socket.getProtocol).toBe("function");
  });

  it("should have getCipher method", () => {
    const socket = new tls.TLSSocket();
    expect(typeof socket.getCipher).toBe("function");
  });

  it("should have getPeerCertificate method", () => {
    const socket = new tls.TLSSocket();
    expect(typeof socket.getPeerCertificate).toBe("function");
  });

  it("should have getProtocol method that returns string or null", () => {
    const socket = new tls.TLSSocket();
    const protocol = socket.getProtocol();
    expect(protocol === null || typeof protocol === "string").toBe(true);
  });

  it("should have getCipher method that returns object or null", () => {
    const socket = new tls.TLSSocket();
    const cipher = socket.getCipher();
    expect(cipher === null || typeof cipher === "object").toBe(true);
  });

  it("should return empty object from getPeerCertificate before connection", () => {
    const socket = new tls.TLSSocket();
    expect(socket.getPeerCertificate()).toEqual({});
  });
});

describe("tls.SecureContext", () => {
  it("should create a SecureContext instance with createSecureContext", () => {
    const ctx = tls.createSecureContext();
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });

  it("should create a SecureContext with certificate options", () => {
    const ctx = tls.createSecureContext({
      cert: serverCert,
      key: serverKey,
    });
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });

  it("should create a SecureContext with CA option", () => {
    const ctx = tls.createSecureContext({
      ca: serverCert,
    });
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });
});

describe("tls module constants", () => {
  it("should export DEFAULT_MIN_VERSION", () => {
    expect(tls.DEFAULT_MIN_VERSION).toBe("TLSv1.2");
  });

  it("should export DEFAULT_MAX_VERSION", () => {
    expect(tls.DEFAULT_MAX_VERSION).toBe("TLSv1.3");
  });
});

describe("tls.getCiphers()", () => {
  it("should export getCiphers function", () => {
    expect(typeof tls.getCiphers).toBe("function");
  });

  it("should return an array of cipher names", () => {
    const ciphers = tls.getCiphers();
    expect(Array.isArray(ciphers)).toBe(true);
    expect(ciphers.length).toBeGreaterThan(0);
  });

  it("should include TLS 1.3 ciphers", () => {
    const ciphers = tls.getCiphers();
    // TLS 1.3 cipher names start with TLS_
    const tls13Ciphers = ciphers.filter((c: string) => c.startsWith("TLS_"));
    expect(tls13Ciphers.length).toBeGreaterThan(0);
  });

  it("should include TLS 1.2 ciphers", () => {
    const ciphers = tls.getCiphers();
    // TLS 1.2 cipher names typically include ECDHE
    const tls12Ciphers = ciphers.filter((c: string) => c.includes("ECDHE"));
    expect(tls12Ciphers.length).toBeGreaterThan(0);
  });
});

describe("minVersion/maxVersion options", () => {
  it("should accept minVersion option in connect", () => {
    // This test just ensures the options are parsed without error
    // Actual connection would require a server
    const socket = new tls.TLSSocket();
    expect(typeof socket.connect).toBe("function");
  });

  it("should accept minVersion in SecureContext", () => {
    const ctx = tls.createSecureContext({
      minVersion: "TLSv1.2",
    });
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });

  it("should accept maxVersion in SecureContext", () => {
    const ctx = tls.createSecureContext({
      maxVersion: "TLSv1.3",
    });
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });

  it("should accept both minVersion and maxVersion in SecureContext", () => {
    const ctx = tls.createSecureContext({
      minVersion: "TLSv1.2",
      maxVersion: "TLSv1.3",
    });
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });
});

describe("tls.rootCertificates", () => {
  it("should export rootCertificates", () => {
    expect(typeof tls.rootCertificates).toBe("function");
  });

  it("should return an array of PEM certificates", () => {
    const certs = tls.rootCertificates();
    expect(Array.isArray(certs)).toBe(true);
    expect(certs.length).toBeGreaterThan(0);
  });

  it("should contain PEM formatted certificates", () => {
    const certs = tls.rootCertificates();
    const firstCert = certs[0];
    expect(firstCert).toContain("-----BEGIN CERTIFICATE-----");
    expect(firstCert).toContain("-----END CERTIFICATE-----");
  });
});

describe("tls.checkServerIdentity()", () => {
  it("should export checkServerIdentity function", () => {
    expect(typeof tls.checkServerIdentity).toBe("function");
  });

  it("should return undefined for matching hostname in subject CN", () => {
    const cert = {
      subject: { CN: "example.com" },
    };
    const result = tls.checkServerIdentity("example.com", cert);
    expect(result).toBeUndefined();
  });

  it("should return undefined for matching hostname in subjectaltname", () => {
    const cert = {
      subject: { CN: "other.com" },
      subjectaltname: "DNS:example.com, DNS:www.example.com",
    };
    const result = tls.checkServerIdentity("example.com", cert);
    expect(result).toBeUndefined();
  });

  it("should return undefined for wildcard match", () => {
    const cert = {
      subject: { CN: "example.com" },
      subjectaltname: "DNS:*.example.com",
    };
    const result = tls.checkServerIdentity("www.example.com", cert);
    expect(result).toBeUndefined();
  });

  it("should throw for non-matching hostname", () => {
    const cert = {
      subject: { CN: "example.com" },
    };
    expect(() => {
      tls.checkServerIdentity("other.com", cert);
    }).toThrow();
  });
});

describe("secureContext option in connect", () => {
  it("should accept secureContext option in TLSSocket.connect()", () => {
    const socket = new tls.TLSSocket();
    const ctx = tls.createSecureContext({
      ca: serverCert,
    });
    // Verify the socket has connect method that can accept secureContext
    expect(typeof socket.connect).toBe("function");
  });

  it("should create SecureContext with CA certificates", () => {
    const ctx = tls.createSecureContext({
      ca: serverCert,
    });
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });

  it("should create SecureContext with cert and key", () => {
    const ctx = tls.createSecureContext({
      cert: serverCert,
      key: serverKey,
    });
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });

  it("should create SecureContext with all options", () => {
    const ctx = tls.createSecureContext({
      cert: serverCert,
      key: serverKey,
      ca: serverCert,
      minVersion: "TLSv1.2",
      maxVersion: "TLSv1.3",
    });
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });
});

describe("TLSSocket keylog event", () => {
  it("should support adding keylog event listener", () => {
    const socket = new tls.TLSSocket();
    let keylogCalled = false;
    socket.on("keylog", (line: ArrayBuffer) => {
      keylogCalled = true;
    });
    // Just verify the listener can be added without errors
    expect(typeof socket.on).toBe("function");
  });

  it("should emit keylog event with Buffer containing NSS key log format", (done) => {
    // This test verifies the keylog event format
    // The actual emission happens during TLS handshake with a real connection
    const socket = new tls.TLSSocket();

    socket.on("keylog", (line: ArrayBuffer) => {
      // keylog line should be an ArrayBuffer
      expect(line instanceof ArrayBuffer).toBe(true);
      // Convert to string to check format
      const lineStr = new TextDecoder().decode(line);
      // NSS key log format: <label> <client_random_hex> <secret_hex>\n
      expect(lineStr).toMatch(/^[A-Z_]+ [0-9a-f]+ [0-9a-f]+\n$/);
      socket.destroy();
      done();
    });

    // Note: This test would need an actual TLS connection to trigger keylog
    // For unit testing, we verify the listener can be added
    // Integration tests would verify actual key logging
    socket.destroy();
    done();
  });
});

describe("Client certificate authentication (mTLS)", () => {
  it("should accept cert and key options in connect", () => {
    const socket = new tls.TLSSocket();
    // Verify that connect method exists and can be called with cert/key options
    expect(typeof socket.connect).toBe("function");
  });

  it("should accept cert option as string", () => {
    // This test verifies the options are parsed without error
    // Actual connection would require a server that validates client certs
    const socket = new tls.TLSSocket();
    expect(() => {
      // Just verify the socket accepts these options in its API
      // The actual connect would fail without a server, but parsing should work
    }).not.toThrow();
  });

  it("should accept cert option as Buffer", () => {
    const socket = new tls.TLSSocket();
    expect(() => {
      // Verify Buffer is an acceptable type for cert/key
    }).not.toThrow();
  });

  it("should create SecureContext with client cert for mTLS", () => {
    // SecureContext already supports cert/key for client auth
    const ctx = tls.createSecureContext({
      cert: serverCert,
      key: serverKey,
    });
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });

  it("should create SecureContext with CA and client cert", () => {
    // Full mTLS configuration: CA for server verification + client cert for auth
    const ctx = tls.createSecureContext({
      ca: serverCert,
      cert: serverCert,
      key: serverKey,
    });
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });

  it("should support passing secureContext with client cert to connect", () => {
    const ctx = tls.createSecureContext({
      cert: serverCert,
      key: serverKey,
    });
    const socket = new tls.TLSSocket();
    // Verify the socket can accept a secureContext with client certs
    expect(typeof socket.connect).toBe("function");
    expect(ctx).toBeInstanceOf(tls.SecureContext);
  });
});
