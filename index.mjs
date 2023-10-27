import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
import { DynamoDBDocumentClient, PutCommand } from "@aws-sdk/lib-dynamodb";

const client = new DynamoDBClient({});
const docClient = DynamoDBDocumentClient.from(client);

export const handler = async (event) => {
  const start = Date.now();
  await docClient.send(
    new PutCommand({
      TableName: process.env.TABLE_NAME,
      Item: {
        id: Math.random().toString(36).substring(2),
        content: JSON.stringify(event),
      },
    })
  );
  const end = Date.now();
  console.log(`Done in ${end - start}ms`);
  return {
    statusCode: 200,
    body: "OK",
  };
};
