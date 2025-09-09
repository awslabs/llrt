import "./sdk-runtime-init.mjs";

//to simulate some async initialization work
await new Promise((r) => setTimeout(r, 100));

export const handler = async () => {
  return {
    statusCode: 200,
    body: "Hello from sdk handler",
  };
};
