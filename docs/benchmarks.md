# Benchmark methodology

Although Init Duration [reported by Lambda](https://docs.aws.amazon.com/lambda/latest/dg/lambda-runtime-environment.html) is commonly used to understand cold start impact on overall request latency, this metric does not include the time needed to copy code into the Lambda sandbox.

The technical definition of Init Duration ([source](https://docs.aws.amazon.com/lambda/latest/dg/nodejs-logging.html#node-logging-output)):

> For the first request served, the amount of time it took the runtime to load the function and run code outside of the handler method.

Measuring round-trip request duration provides a more complete picture of user facing cold-start latency.

Lambda invocation results (λ-labeled row) report the sum total of Init Duration + Function Duration.
