import {
  DynamoDBDocumentClient,
  PutCommand,
  ScanCommand,
  DeleteCommand,
  UpdateCommand,
} from "@aws-sdk/lib-dynamodb";
import { DynamoDBClient } from "@aws-sdk/client-dynamodb";
import { randomBytes } from "crypto";
import { Todo } from "./react/TodoList";

const uid = () =>
  String.fromCharCode(
    ...randomBytes(20).map((d) => {
      return (d > 127 ? 97 : 65) + (d % 25);
    })
  ) + new Date().getTime();

const CLIENT = new DynamoDBClient({});
const DOCUMENT_CLIENT = DynamoDBDocumentClient.from(CLIENT as any);

const mapTodo = (
  item: Todo
): {
  id: string; // Assuming id is a string attribute
  text: string; // Assuming text is a string attribute
  createdDate: string; // Assuming createdDate is a number attribute
  completedDate: string | null;
} => ({
  ...item,
  createdDate: new Date(parseInt(item.createdDate)).toISOString(),
  completedDate: item.completedDate
    ? new Date(parseInt(item.completedDate)).toISOString()
    : null,
});

const API = {
  getAll: async () => {
    const response = await DOCUMENT_CLIENT.send(
      new ScanCommand({
        TableName: process.env.TABLE_NAME,
      })
    );
    console.log(response);
    const items = response.Items.map(mapTodo);
    return items;
  },
  create: async (text: string) => {
    const newItem = {
      id: uid(),
      text,
      createdDate: Date.now(),
    };
    await DOCUMENT_CLIENT.send(
      new PutCommand({
        TableName: process.env.TABLE_NAME,
        Item: newItem,
      })
    );
    return newItem;
  },

  delete: async (id: string) => {
    await DOCUMENT_CLIENT.send(
      new DeleteCommand({
        TableName: process.env.TABLE_NAME,
        Key: {
          id,
        },
      })
    );
  },
  update: async (todo: Omit<Todo, "createdDate">) => {
    await DOCUMENT_CLIENT.send(
      new UpdateCommand({
        TableName: process.env.TABLE_NAME,
        Key: {
          id: todo.id,
        },
        UpdateExpression: "set completedDate = :completedDate",
        ExpressionAttributeValues: {
          ":completedDate": todo.completedDate ? Date.now() : null,
        },
      })
    );
    return todo;
  },
};

export default API;
