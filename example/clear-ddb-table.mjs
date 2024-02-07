import {
  DynamoDBClient,
  ScanCommand,
  DescribeTableCommand,
  BatchWriteItemCommand,
} from "@aws-sdk/client-dynamodb";
import path from "path";

const DDB_CLIENT = new DynamoDBClient({});

const TABLE_ARN = process.argv[2];

if (!TABLE_ARN) {
  console.error(
    `Usage: ${path.basename(process.argv0)} ${path.basename(
      process.argv[1]
    )} <table-arn>`
  );
  process.exit(1);
}

const segmentArray = (array, segmentSize) =>
  Array.from({ length: Math.ceil(array.length / segmentSize) }, (_, index) =>
    array.slice(index * segmentSize, (index + 1) * segmentSize)
  );

function extractRegionAndTableName(arn) {
  const parts = arn.split(":");

  if (
    parts.length >= 6 &&
    parts[2] === "dynamodb" &&
    parts[5].startsWith("table/")
  ) {
    const region = parts[3];
    const tableName = parts[5].substring(6);
    return { region, tableName };
  } else {
    return null;
  }
}

const sleep = (time) => new Promise((resolve) => setTimeout(resolve, time));

class AsyncProcessor {
  constructor(producerFunction, consumerFunction) {
    this.producerFunction = producerFunction;
    this.consumerFunction = consumerFunction;
  }

  static emptyJob() {
    let resolveFn;
    let rejectFn;
    const promise = new Promise((resolve, reject) => {
      resolveFn = resolve;
      rejectFn = reject;
    });
    return [promise, resolveFn, rejectFn];
  }

  async process() {
    let error;

    const jobs = [AsyncProcessor.emptyJob()];

    const producer = async () => {
      let currentJob = jobs[0];
      let nextJob;
      try {
        while (!error) {
          let data = await this.producerFunction();
          if (data) {
            nextJob = AsyncProcessor.emptyJob();
            jobs.push(nextJob);
          }
          currentJob[1](data);
          if (!data) {
            break;
          }

          currentJob = nextJob;
        }
      } catch (e) {
        error = e;
        throw e;
      }
    };

    const consumer = async () => {
      try {
        while (!error) {
          const job = jobs.shift();
          const data = await job[0];
          if (!data) {
            break;
          }
          await this.consumerFunction(data);
        }
      } catch (e) {
        error = e;
        throw e;
      }
    };

    await Promise.all([producer(), consumer()]);
  }
}

async function deleteItems(tableName, primaryKey, keys) {
  let deleteRequests = keys.map((key) => ({
    DeleteRequest: {
      Key: { [primaryKey]: key },
    },
  }));
  let attempt = 0;

  if (deleteRequests.length === 0) {
    return;
  }

  const start = Date.now();
  while (true) {
    const result = await DDB_CLIENT.send(
      new BatchWriteItemCommand({
        RequestItems: {
          [tableName]: deleteRequests,
        },
      })
    );
    if (
      result.UnprocessedItems[tableName] &&
      result.UnprocessedItems[tableName].length > 0
    ) {
      deleteRequests = result.UnprocessedItems[tableName];
      attempt++;
      if (attempt > 3) {
        throw new Error("Too many attempts");
      }
      await sleep(500 * Math.pow(attempt, 2));
    } else {
      break;
    }
  }
  console.log(`Deleted:  ${Date.now() - start}ms`);
}

async function clearDynamoDBTable() {
  const { region, tableName } = extractRegionAndTableName(TABLE_ARN);
  process.env.AWS_REGION = region;

  const primaryKey = await getPrimaryKey(tableName);

  let exclusiveStartKey = undefined;
  let totalCount = 0;

  const processor = new AsyncProcessor(
    async () => {
      const start = Date.now();
      const scanOutput = await DDB_CLIENT.send(
        new ScanCommand({
          TableName: tableName,
          ProjectionExpression: primaryKey,
          ExclusiveStartKey: exclusiveStartKey,
        })
      );
      console.log(`Scanned:  ${Date.now() - start}ms`);

      exclusiveStartKey = scanOutput.LastEvaluatedKey;

      const ids = scanOutput.Items.map((item) => item[primaryKey]);
      if (ids.length === 0) {
        return null;
      }
      return ids;
    },
    async (keys) => {
      const segmentedKeys = segmentArray(keys, 25);
      await Promise.all(
        segmentedKeys.map(async (keys) => {
          await deleteItems(tableName, primaryKey, keys);
          totalCount += keys.length;
          if (totalCount > 10000) {
            process.exit(0);
          }
          console.log("Deleted:", totalCount);
        })
      );
    }
  );

  await processor.process();
}

async function getPrimaryKey(tableName) {
  const describeOutput = await DDB_CLIENT.send(
    new DescribeTableCommand({
      TableName: tableName,
    })
  );
  return describeOutput.Table.KeySchema.find((key) => key.KeyType === "HASH")
    .AttributeName;
}

clearDynamoDBTable().catch((err) => {
  console.error(err);
  process.exit(1);
});
