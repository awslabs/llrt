class LRUCache {
  constructor(capacity = 10) {
    this.capacity = capacity;
    this.cache = new Map();
  }

  get(key) {
    let item = this.cache.get(key);
    if (item) {
      // refresh key
      this.cache.delete(key);
      this.cache.set(key, item);
    }
    return item;
  }

  set(key, val) {
    if (this.cache.has(key)) {
      this.cache.delete(key);
    } else if (this.cache.size == this.capacity) {
      this.cache.delete(this.first());
    }
    this.cache.set(key, val);
  }

  has(key) {
    return this.cache.has(key);
  }

  first() {
    return this.cache.keys().next().value;
  }

  clear() {
    this.cache.clear();
  }
}

export default LRUCache;
