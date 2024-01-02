const delay = (time) => new Promise((res) => setTimeout(res, time));
const tasks = [];

const numTasks = 1000 * 10;

for (let i = 0; i < numTasks; i++) {
  tasks.push(delay(1000));
}

(async function () {
  console.log(1);
  await Promise.all(tasks);
  console.log(2);
})();

//async () => Promise.all(tasks))().catch(console.log);

// globalThis.ReadableStream = class ReadableStream {};
// import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3";
// import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
// import { DynamoDBDocumentClient, PutCommand } from "@aws-sdk/lib-dynamodb";
// import fs from "fs/promises";

// import { randomBytes } from "crypto";

// const uid = () =>
//   String.fromCharCode(
//     ...randomBytes(10).map((d) => {
//       return (d > 127 ? 97 : 65) + (d % 25);
//     })
//   );

// const s3Client = new S3Client({});
// const dynamoDbClient = new DynamoDBClient({});
// const docClient = DynamoDBDocumentClient.from(dynamoDbClient);

// export const handler = async (event) => {
//   let id = uid();
//   let data = JSON.stringify(event);

//   await s3Client.send(
//     new PutObjectCommand({
//       Body: data,
//       Bucket: process.env.BUCKET_NAME,
//       Key: id,
//     })
//   );

//   // await docClient.send(
//   //   new PutCommand({
//   //     TableName: process.env.TABLE_NAME,
//   //     Item: {
//   //       id,
//   //       content: "example_content",
//   //     },
//   //   })
//   // );

//   return {
//     statusCode: 200,
//     body: "OK",
//   };
// };

// const main = async () => {
//   const data = JSON.parse(await fs.readFile("slask/data2.json"));

//   for (let i = 0; i < 10000; i++) {
//     console.log("running ", i);
//     await handler(data);
//   }
// };

// main().catch(console.log);
