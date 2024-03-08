// From https://github.com/aws/aws-sdk-js-v3/blob/c8cb4499c6ad19e2b194860d548753f637671f8c/clients/client-cognito-identity/test/e2e/CognitoIdentity.ispec.ts#L6

import { expect } from "chai";
import { CognitoIdentity } from "@aws-sdk/client-cognito-identity";

const IdentityPoolId = process?.env?.AWS_SMOKE_TEST_IDENTITY_POOL_ID;

describe("@aws-sdk/client-cognito-identity", function () {
  const unAuthClient = new CognitoIdentity({});

  it("should successfully fetch Id and get credentials", async () => {
    // Test getId()
    const getIdResult = await unAuthClient.getId({
      IdentityPoolId,
    });
    expect(getIdResult.$metadata.httpStatusCode).to.equal(200);
    expect(typeof getIdResult.IdentityId).to.equal("string");

    // Test getCredentialsForIdentity() with Id from above
    const getCredentialsResult = await unAuthClient.getCredentialsForIdentity({
      IdentityId: getIdResult.IdentityId,
    });
    expect(getCredentialsResult.$metadata.httpStatusCode).to.equal(200);
    expect(typeof getCredentialsResult.Credentials?.AccessKeyId).to.equal(
      "string"
    );
    expect(typeof getCredentialsResult.Credentials?.SecretKey).to.equal(
      "string"
    );
  });
});
