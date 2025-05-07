// META: title=WebCryptoAPI: CryptoKey cached ECMAScript objects

// https://w3c.github.io/webcrypto/#dom-cryptokey-algorithm
// https://github.com/servo/servo/issues/33908

export default function(ctx) {
const { promise_test, assert_unreached, assert_true } = ctx;

  promise_test(function() {
      return self.crypto.subtle.generateKey(
          {
            name: "AES-CTR",
            length: 256,
          },
          true,
          ["encrypt"],
        ).then(
          function(key) {
            let a = key.algorithm;
            let b = key.algorithm;
            assert_true(a === b);
          },
          function(err) {
              assert_unreached("generateKey threw an unexpected error: " + err.toString());
          }
      );
  }, "CryptoKey.algorithm getter returns cached object");

  promise_test(function() {
      return self.crypto.subtle.generateKey(
          {
            name: "AES-CTR",
            length: 256,
          },
          true,
          ["encrypt"],
        ).then(
          function(key) {
            let a = key.usages;
            let b = key.usages;
            assert_true(a === b);
          },
          function(err) {
              assert_unreached("generateKey threw an unexpected error: " + err.toString());
          }
      );
  }, "CryptoKey.usages getter returns cached object");
}
