let initialized = false;

Object.defineProperty(exports, "lazy", {
  enumerable: true,
  get() {
    initialized = true;
    return 42;
  },
});

exports.wasInitialized = () => initialized;
