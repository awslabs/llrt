const a = require("./import.cjs");

exports.handler = async () => {
  return {
    statusCode: 200,
    body: "OK",
  };
};
