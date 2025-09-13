await fetch(__MOCK_ENDPOINT);

export const handler = async () => {
  return {
    statusCode: 200,
    body: "OK",
  };
};
