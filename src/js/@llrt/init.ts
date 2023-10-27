__bootstrap.initTasks = [];
const initTasks = __bootstrap.initTasks;
__bootstrap.addInitTask = (task: Promise<any>) => {
  initTasks.push(task);
};

__bootstrap.invokeHandler = async (event: any) =>
  global.__handler(event).then(JSON.stringify);

const REGION = process.env.AWS_REGION || "us-east-1";

__bootstrap.addAwsSdkInitTask = (service: string) => {
  const start = Date.now();
  const connectTask = fetch(`https://${service}.${REGION}.amazonaws.com`, {
    method: "GET",
  }).then(() => {
    console.log("INIT_CONNECTION", service, `${Date.now() - start}ms`);
  });
  initTasks.push(connectTask);
};
