export const fromBase64 = (input) => Buffer.from(input, "base64");
export const toBase64 = (input) => Buffer.from(input).toString("base64");
