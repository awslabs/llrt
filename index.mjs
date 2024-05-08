console.log(123);

let i = setInterval(() => {
  console.log("interval");
}, 1000);

setTimeout(() => {
  console.log("timeout");
  clearInterval(i);
}, 5000);

await Promise.resolve(1);

// setTimeout(() => {
//   console.log("timeout");
// }, 1000);

//let signal = AbortSignal.timeout(5);

// console.log(new Promise(() => {}));
// console.log(Promise.resolve(new Array(10000).fill("a")));

// // import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
// // import { DynamoDBDocumentClient, PutCommand } from "@aws-sdk/lib-dynamodb";

// // const client = new DynamoDBClient({});
// // const docClient = DynamoDBDocumentClient.from(client);

// // export const handler = async (event) => {
// //   await docClient.send(
// //     new PutCommand({
// //       TableName: process.env.TABLE_NAME,
// //       Item: {
// //         id: Math.random().toString(36).substring(2),
// //         content: JSON.stringify(event),
// //       },
// //     })
// //   );
// //   return {
// //     statusCode: 200,
// //     body: "OK",
// //   };
// // };
