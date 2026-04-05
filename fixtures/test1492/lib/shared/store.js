const store = {};

console.log("store initialized");

export function set(key, value) {
  store[key] = value;
}

export function get(key) {
  return store[key];
}
