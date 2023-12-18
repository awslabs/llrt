import {
  App,
  aws_dynamodb,
  aws_lambda,
  aws_lambda_nodejs,
  aws_s3,
  aws_logs,
  aws_cloudfront,
  aws_cloudfront_origins,
  aws_apigatewayv2,
  aws_apigatewayv2_integrations,
  CfnOutput,
  Stack,
  Fn,
  Duration,
} from "aws-cdk-lib";
import * as fs from "fs/promises";
import os from "os";
import path from "path";
import { execSync } from "child_process";

const main = async () => {
  execSync("node build.mjs", {
    cwd: "../functions",
    stdio: "inherit",
  });

  const app = new App();
  const stack = new Stack(app, "llrt-example", {
    env: {
      region: process.env.CDK_DEFAULT_REGION,
    },
  });

  const routePaths: string[] = [];

  const tmpDir = os.tmpdir();
  const targetFunctionsDir = path.join(tmpDir, "functions");
  const sourceFunctionsDir = path.resolve("../functions/src");

  await fs.mkdir(targetFunctionsDir, { recursive: true });
  const sourceDirs = {};
  const sources = await fs.readdir(sourceFunctionsDir);
  await Promise.all(
    sources.map(async (source) => {
      if (source === "react") {
        return;
      }
      const { name, ext } = path.parse(source);
      const targetDir = path.join(targetFunctionsDir, name);
      await fs.mkdir(targetDir, { recursive: true });
      await fs.copyFile(
        path.join(sourceFunctionsDir, source),
        path.join(targetDir, `index${ext}`)
      );
      sourceDirs[source] = targetDir;
    })
  );

  const httpApi = new aws_apigatewayv2.HttpApi(stack, `HttpApi`, {
    disableExecuteApiEndpoint: false,
    corsPreflight: undefined,
  });

  const httpEndpointNoProto = Fn.select(
    1,
    Fn.split("://", httpApi.apiEndpoint)
  );

  const createDistribution = (route: string) => {
    const id = route.substring(1).replace(/-\//g, "");
    const distribution = new aws_cloudfront.Distribution(
      stack,
      `${id}Distribution`,
      {
        defaultBehavior: {
          origin: new aws_cloudfront_origins.HttpOrigin(httpEndpointNoProto, {
            protocolPolicy: aws_cloudfront.OriginProtocolPolicy.HTTPS_ONLY,
            originPath: route,
          }),
          allowedMethods: aws_cloudfront.AllowedMethods.ALLOW_ALL,
          cachePolicy: aws_cloudfront.CachePolicy.CACHING_DISABLED,
        },
      }
    );
    new CfnOutput(stack, `DistributionOutput${id}`, {
      value: distribution.distributionDomainName,
    });
  };

  const addRoute = (lambda: aws_lambda.Function, routePath: string) => {
    const integration = new aws_apigatewayv2_integrations.HttpLambdaIntegration(
      `${lambda.node.id}Integration`,
      lambda
    );

    new aws_apigatewayv2.HttpRoute(stack, `${lambda.node.id}Route`, {
      httpApi,
      routeKey: aws_apigatewayv2.HttpRouteKey.with(
        routePath,
        aws_apigatewayv2.HttpMethod.ANY
      ),
      integration,
    });
    new aws_apigatewayv2.HttpRoute(stack, `${lambda.node.id}ProxyRoute`, {
      httpApi,
      routeKey: aws_apigatewayv2.HttpRouteKey.with(
        `${routePath}/{proxy+}`,
        aws_apigatewayv2.HttpMethod.ANY
      ),
      integration,
    });

    routePaths.push(routePath);
  };

  const table = new aws_dynamodb.Table(stack, "Table", {
    partitionKey: {
      name: "id",
      type: aws_dynamodb.AttributeType.STRING,
    },
    billingMode: aws_dynamodb.BillingMode.PAY_PER_REQUEST,
  });

  const todoTable = new aws_dynamodb.Table(stack, "TodoTable", {
    partitionKey: {
      name: "id",
      type: aws_dynamodb.AttributeType.STRING,
    },
    billingMode: aws_dynamodb.BillingMode.PAY_PER_REQUEST,
  });

  const bucket = new aws_s3.Bucket(stack, "Bucket", {});

  const props = {
    environment: {
      TABLE_NAME: table.tableName,
      BUCKET_NAME: bucket.bucketName,
    },
    runtime: aws_lambda.Runtime.NODEJS_18_X,
    memorySize: 128,
    timeout: Duration.seconds(60),
    handler: "index.handler",
    bundling: {
      format: aws_lambda_nodejs.OutputFormat.ESM,
      banner:
        "import {createRequire} from 'module';const require=createRequire(import.meta.url);",
      minify: true,
      sourceMap: true,
    },
    architecture: aws_lambda.Architecture.ARM_64,
    logRetention: aws_logs.RetentionDays.ONE_WEEK,
  };

  const llrtLayer = new aws_lambda.LayerVersion(stack, "LlrtArmLayer", {
    code: aws_lambda.Code.fromAsset("../../llrt-lambda-arm64.zip"),
    compatibleRuntimes: [
      aws_lambda.Runtime.NODEJS_16_X,
      aws_lambda.Runtime.NODEJS_18_X,
      aws_lambda.Runtime.NODEJS_LATEST,
      aws_lambda.Runtime.PROVIDED_AL2,
    ],
    compatibleArchitectures: [aws_lambda.Architecture.ARM_64],
  });

  // LLRT hello
  const helloLlrtFunction = new aws_lambda.Function(
    stack,
    "HelloLlrtFunction",
    {
      functionName: "example-hello-llrt",
      code: aws_lambda.Code.fromAsset(sourceDirs["hello.mjs"]),
      ...props,
      environment: {},
      runtime: aws_lambda.Runtime.PROVIDED_AL2,
      layers: [llrtLayer],
    }
  );

  // Node 18 hello
  const helloNode18Function = new aws_lambda.Function(
    stack,
    "HelloNode18Function",
    {
      functionName: "example-hello-node18",
      code: aws_lambda.Code.fromAsset(sourceDirs["hello.mjs"]),
      ...props,
      environment: {},
    }
  );

  const helloNode16Function = new aws_lambda.Function(
    stack,
    "HelloNode16Function",
    {
      functionName: "example-hello-node16",
      code: aws_lambda.Code.fromAsset(sourceDirs["hello.mjs"]),
      ...props,
      runtime: aws_lambda.Runtime.NODEJS_16_X,
      environment: {},
    }
  );

  // Node 18, provided "aws-sdk"
  const v2Function = new aws_lambda_nodejs.NodejsFunction(stack, "V2", {
    functionName: "example-v2",
    entry: "../functions/src/v2.mjs",
    ...props,
    bundling: {
      ...props.bundling,
      externalModules: ["aws-sdk"],
    },
  });

  // Node 18, aws-sdk-v3, DynamoDBClient.send API, bundled in
  const v3BundledFunction = new aws_lambda_nodejs.NodejsFunction(
    stack,
    "V3Bundled",
    {
      functionName: "example-v3-bundled",
      entry: "../functions/src/v3.mjs",
      ...props,
      bundling: {
        ...props.bundling,
        externalModules: [],
      },
    }
  );

  // Node 18, aws-sdk-v3, DynamoDBClient.send API, tree-shaken out (using one provided by us)
  const v3providedFunction = new aws_lambda.Function(stack, "V3Provided", {
    functionName: "example-v3-provided",
    code: aws_lambda.Code.fromAsset(sourceDirs["v3.mjs"]),
    ...props,
  });

  // Node 18, aws-sdk-v3, DynamoDB.putItem (mono API), tree-shaken
  const v3providedMonoFunction = new aws_lambda_nodejs.NodejsFunction(
    stack,
    "V3providedMono",
    {
      functionName: "example-v3-mono-provided",
      entry: "../functions/src/v3-mono.mjs",
      ...props,
      bundling: {
        ...props.bundling,
        externalModules: ["@aws-sdk/*"],
      },
    }
  );

  // Node 18, aws-sdk-v3, DynamoDB.putItem (mono API), bundled
  const v3BundledMonoFunction = new aws_lambda_nodejs.NodejsFunction(
    stack,
    "V3BundledMono",
    {
      functionName: "example-v3-mono-bundled",
      entry: "../functions/src/v3-mono.mjs",
      ...props,
      bundling: {
        ...props.bundling,
        externalModules: [],
      },
    }
  );

  // LLRT aws-sdk-v3, DynamoDBClient.send API
  const llrtFunction = new aws_lambda.Function(stack, "LlrtFunction", {
    functionName: "example-llrt",
    code: aws_lambda.Code.fromAsset(sourceDirs["v3-lib.mjs"]),
    handler: "index.handler",
    ...props,
    runtime: aws_lambda.Runtime.PROVIDED_AL2,
    layers: [llrtLayer],
  });

  // LLRT aws-sdk-v3, DynamoDBClient & S3 API
  const llrtS3Function = new aws_lambda.Function(stack, "LlrtS3Function", {
    functionName: "example-llrt-s3",
    code: aws_lambda.Code.fromAsset(sourceDirs["v3-s3.mjs"]),
    handler: "index.handler",
    ...props,
    runtime: aws_lambda.Runtime.PROVIDED_AL2,
    environment: {
      ...props.environment,
    },
    layers: [llrtLayer],
  });

  // aws-sdk-v3, DynamoDBClient & S3 API
  const s3Function = new aws_lambda.Function(stack, "S3Function", {
    functionName: "example-s3",
    code: aws_lambda.Code.fromAsset(sourceDirs["v3-s3.mjs"]),
    handler: "index.handler",
    ...props,
    environment: {
      ...props.environment,
    },
  });

  // ssr react, node.js
  const reactFunction = new aws_lambda.Function(stack, "ReactFunction", {
    functionName: "example-react",
    code: aws_lambda.Code.fromAsset("../functions/build"),
    handler: "index.handler",
    ...props,
    environment: {
      ...props.environment,
      TABLE_NAME: todoTable.tableName,
    },
  });

  // ssr react,llrt
  const llrtReactFunction = new aws_lambda.Function(
    stack,
    "LlrtReactFunction",
    {
      functionName: "example-llrt-react",
      code: aws_lambda.Code.fromAsset("../functions/build"),
      handler: "index.handler",
      ...props,
      runtime: aws_lambda.Runtime.PROVIDED_AL2,
      environment: {
        ...props.environment,
        TABLE_NAME: todoTable.tableName,
      },
      layers: [llrtLayer],
    }
  );

  todoTable.grantReadWriteData(reactFunction);
  todoTable.grantReadWriteData(llrtReactFunction);
  table.grantReadWriteData(v2Function);
  table.grantReadWriteData(v3BundledFunction);
  table.grantReadWriteData(v3providedFunction);
  table.grantReadWriteData(v3providedMonoFunction);
  table.grantReadWriteData(v3BundledMonoFunction);
  table.grantReadWriteData(llrtFunction);
  table.grantReadWriteData(llrtS3Function);
  table.grantReadWriteData(s3Function);

  bucket.grantReadWrite(llrtS3Function);
  bucket.grantReadWrite(s3Function);

  addRoute(helloNode16Function, "/hello-16");
  addRoute(helloNode18Function, "/hello-18");
  addRoute(helloLlrtFunction, "/hello-llrt");
  addRoute(v2Function, "/v2");
  addRoute(v3BundledFunction, "/v3-bundled");
  addRoute(v3providedFunction, "/v3-provided");
  addRoute(v3providedMonoFunction, "/v3-provided-mono");
  addRoute(v3BundledMonoFunction, "/v3-bundled-mono");
  addRoute(llrtFunction, "/llrt");
  addRoute(llrtS3Function, "/llrt-s3");
  addRoute(s3Function, "/s3");
  addRoute(reactFunction, "/react");
  addRoute(llrtReactFunction, "/llrt-react");

  for (const [i, route] of routePaths.entries()) {
    new CfnOutput(stack, `HttpApiOutput${i}`, {
      value: `${httpApi.apiEndpoint}${route}`,
    });
  }

  createDistribution("/llrt-react");
  createDistribution("/react");
};

main();
