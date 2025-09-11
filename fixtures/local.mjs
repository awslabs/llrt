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

const clientCfg = {
  endpoint: "http://localhost:8080/service/",
};

const client = new DynamoDBClient(clientCfg);
const docClient = DynamoDBDocumentClient.from(client);
const s3Client = new S3Client(clientCfg);

export const handler = async (event) => {
  const start = Date.now();

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

  const end = Date.now();
  const time = `Duration: ${end - start}ms`;
  console.log(time);
  return {
    statusCode: 200,
    body: time,
  };
};
