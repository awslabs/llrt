import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
import { DynamoDBDocumentClient, PutCommand } from "@aws-sdk/lib-dynamodb";
import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3";

import { randomBytes } from "node:crypto";

const uid = () =>
  String.fromCharCode(
    ...randomBytes(10).map((d) => {
      return (d > 127 ? 97 : 65) + (d % 25);
    })
  );

const dynamoDbClient = new DynamoDBClient({});
const s3Client = new S3Client({});
const docClient = DynamoDBDocumentClient.from(dynamoDbClient);

export const handler = async (event) => {
  let id = uid();
  let data = JSON.stringify(event);

  await Promise.all([
    docClient.send(
      new PutCommand({
        TableName: process.env.TABLE_NAME,
        Item: {
          id,
          content: data,
        },
      })
    ),
    s3Client.send(
      new PutObjectCommand({
        Body: data,
        Bucket: process.env.BUCKET_NAME,
        Key: id,
      })
    ),
  ]);

  return {
    statusCode: 200,
    body: "OK",
  };
};
