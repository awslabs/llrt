// Import the necessary AWS SDK clients and commands
import { EC2Client, DescribeInstancesCommand } from "@aws-sdk/client-ec2";

// Create an EC2 client
const client = new EC2Client();

// Lambda handler function
export const handler = async () => {
  const command = new DescribeInstancesCommand({});

  // Send the command to the EC2 client
  const response = await client.send(command);

  // Extract instances information
  const instances = response.Reservations.flatMap(
    (reservation) => reservation.Instances
  );

  // Return the list of instances
  return {
    statusCode: 200,
    body: JSON.stringify(instances),
  };
};
