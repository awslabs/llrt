import { get } from "../shared/store.js";

export function get_in_bar(key) {
  return get(key);
}
