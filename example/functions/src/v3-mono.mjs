import { DynamoDB } from "@aws-sdk/client-dynamodb";

const client = new DynamoDB({});

export const handler = async (event) => {
  await client.putItem({
    TableName: process.env.TABLE_NAME,
    Item: {
      id: {
        S: Math.random().toString(36).substring(2),
      },
      content: {
        S: JSON.stringify(event),
      },
    },
  });
  return {
    statusCode: 200,
    body: "OK",
  };
};
