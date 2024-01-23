import { spawn } from "child_process";

const command = "nonexistent-command";
const child = spawn(command);
//setTimeout(() => {
child.on("error", (err) => {
  console.log("apa");
});
//}, 0);

// import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
// import { DynamoDBDocumentClient, PutCommand } from "@aws-sdk/lib-dynamodb";

// const client = new DynamoDBClient({});
// const docClient = DynamoDBDocumentClient.from(client);

// export const handler = async (event) => {
//   await docClient.send(
//     new PutCommand({
//       TableName: process.env.TABLE_NAME,
//       Item: {
//         id: Math.random().toString(36).substring(2),
//         content: JSON.stringify(event),
//       },
//     })
//   );
//   return {
//     statusCode: 200,
//     body: "OK",
//   };
// };
