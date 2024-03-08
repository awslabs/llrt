// From https://github.com/aws/aws-sdk-js-v3/blob/89f97b5cea8052510471cdad69acced9f5be60d1/clients/client-s3/test/e2e/S3.e2e.spec.ts#L15

import { S3, SelectObjectContentEventStream } from "@aws-sdk/client-s3";
const Bucket = process?.env?.AWS_SMOKE_TEST_BUCKET;
const mrapArn = process?.env?.AWS_SMOKE_TEST_MRAP_ARN;

let Key = `${Date.now()}`;

describe("@aws-sdk/client-s3", () => {
  const client = new S3();

  // TODO Stream is not yet supported
  describe.skip("PutObject", () => {
    beforeAll(() => {
      Key = `${Date.now()}`;
    });
    afterAll(async () => {
      await client.deleteObject({ Bucket, Key });
    });
    it("should succeed with Node.js readable stream body", async () => {
      const length = 10 * 1000; // 10KB
      const chunkSize = 10;
      const { Readable } = require("stream");
      let sizeLeft = length;
      const inputStream = new Readable({
        read() {
          if (sizeLeft <= 0) {
            this.push(null); //end stream;
            return;
          }
          let chunk = "";
          for (let i = 0; i < Math.min(sizeLeft, chunkSize); i++) {
            chunk += "x";
          }
          this.push(chunk);
          sizeLeft -= chunk.length;
        },
      });
      inputStream.size = length; // This is required
      const result = await client.putObject({
        Bucket,
        Key,
        Body: inputStream,
      });
      expect(result.$metadata.httpStatusCode).toEqual(200);
    });
  });

  describe("GetObject", function () {
    beforeAll(async () => {
      Key = `${Date.now()}`;
    });

    afterAll(async () => {
      await client.deleteObject({ Bucket, Key });
    });

    it("should succeed with valid body payload", async () => {
      // prepare the object.
      const body = createBuffer("1MB");

      try {
        await client.putObject({ Bucket, Key, Body: body });
      } catch (e) {
        console.error("failed to put");
        throw e;
      }

      try {
        // eslint-disable-next-line no-var
        var result = await client.getObject({ Bucket, Key });
      } catch (e) {
        console.error("failed to get");
        throw e;
      }

      let actual = result.$metadata.httpStatusCode;
      expect(actual).toEqual(200);
    });
  });

  // TODO FB Open a bug
  describe("ListObjects", () => {
    beforeAll(async () => {
      Key = `${Date.now()}`;
      await client.putObject({ Bucket, Key, Body: "foo" });
    });
    afterAll(async () => {
      await client.deleteObject({ Bucket, Key });
    });
    it("should succeed with valid bucket", async () => {
      const result = await client.listObjects({
        Bucket,
      });
      expect(result.$metadata.httpStatusCode).toEqual(200);
      expect(result.Contents instanceof Array).toEqual(true);
    });

    it("should throw with invalid bucket", async () => {
      try {
        await client.listObjects({ Bucket: "invalid-bucket" });
        assert(false, "Should throw an exception");
      } catch (ignored) {
        console.log("Exception should be thrown");
      }
    });
  });

  describe("MultipartUpload", () => {
    let UploadId: string;
    let Etag: string;
    const multipartObjectKey = `${Key}-multipart`;
    beforeAll(() => {
      Key = `${Date.now()}`;
    });
    afterEach(async () => {
      if (UploadId) {
        await client.abortMultipartUpload({
          Bucket,
          Key: multipartObjectKey,
          UploadId,
        });
      }
      await client.deleteObject({
        Bucket,
        Key: multipartObjectKey,
      });
    });

    it("should successfully create, upload list and complete", async () => {
      //create multipart upload
      const createResult = await client.createMultipartUpload({
        Bucket,
        Key: multipartObjectKey,
      });
      expect(createResult.$metadata.httpStatusCode).toEqual(200);
      expect(typeof createResult.UploadId).toEqual("string");
      UploadId = createResult.UploadId as string;

      //upload part
      const uploadResult = await client.uploadPart({
        Bucket,
        Key: multipartObjectKey,
        UploadId,
        PartNumber: 1,
        Body: createBuffer("1KB"),
      });
      expect(uploadResult.$metadata.httpStatusCode).toEqual(200);
      expect(typeof uploadResult.ETag).toEqual("string");
      Etag = uploadResult.ETag as string;

      //list parts
      const listPartsResult = await client.listParts({
        Bucket,
        Key: multipartObjectKey,
        UploadId,
      });
      expect(listPartsResult.$metadata.httpStatusCode).toEqual(200);
      expect(listPartsResult.Parts?.length).toEqual(1);
      expect(listPartsResult.Parts?.[0].ETag).toEqual(Etag);

      //complete multipart upload // TODO FB bug here
      const completeResult = await client.completeMultipartUpload({
        Bucket,
        Key: multipartObjectKey,
        UploadId,
        MultipartUpload: { Parts: [{ ETag: Etag, PartNumber: 1 }] },
      });
      expect(completeResult.$metadata.httpStatusCode).toEqual(200);

      //validate the object is uploaded
      const headResult = await client.headObject({
        Bucket,
        Key: multipartObjectKey,
      });
      expect(headResult.$metadata.httpStatusCode).toEqual(200);
    });

    it("should successfully create, abort, and list upload", async () => {
      //create multipart upload
      const createResult = await client.createMultipartUpload({
        Bucket,
        Key: multipartObjectKey,
      });
      expect(createResult.$metadata.httpStatusCode).toEqual(200);
      const toAbort = createResult.UploadId;
      expect(typeof toAbort).toEqual("string");

      //abort multipart upload
      const abortResult = await client.abortMultipartUpload({
        Bucket,
        Key: multipartObjectKey,
        UploadId: toAbort,
      });
      expect(abortResult.$metadata.httpStatusCode).toEqual(204);

      //validate multipart upload is aborted // TODO FB bug here
      const listUploadsResult = await client.listMultipartUploads({
        Bucket,
      });
      expect(listUploadsResult.$metadata.httpStatusCode).toEqual(200);
      expect(
        (listUploadsResult.Uploads || []).map((upload) => upload.UploadId)
      ).not.toContain(toAbort);
    });
  });

  // TODO Stream is not yet supported
  describe.skip("selectObjectContent", () => {
    const csvFile = `user_name,age
jsrocks,13
node4life,22
esfuture,29`;
    beforeAll(async () => {
      Key = `${Date.now()}`;
      await client.putObject({ Bucket, Key, Body: csvFile });
    });
    afterAll(async () => {
      await client.deleteObject({ Bucket, Key });
    });
    it("should succeed", async () => {
      const { Payload } = await client.selectObjectContent({
        Bucket,
        Key,
        ExpressionType: "SQL",
        Expression:
          "SELECT user_name FROM S3Object WHERE cast(age as int) > 20",
        InputSerialization: {
          CSV: {
            FileHeaderInfo: "USE",
            RecordDelimiter: "\n",
            FieldDelimiter: ",",
          },
        },
        OutputSerialization: {
          CSV: {},
        },
      });

      const events: SelectObjectContentEventStream[] = [];
      for await (const event of Payload!) {
        events.push(event);
      }
      expect(events.length).toEqual(3);
      expect(new TextDecoder().decode(events[0].Records?.Payload)).toEqual(
        "node4life\nesfuture\n"
      );
      // expect(events[1].Stats?.Details).toBeDefined();
      // expect(events[2].End).toBeDefined();
    });
  });

  describe.skip("Multi-region access point", () => {
    // TODO FB
    beforeAll(async () => {
      Key = `${Date.now()}`;
      await client.putObject({ Bucket: mrapArn, Key, Body: "foo" });
    });
    afterAll(async () => {
      await client.deleteObject({ Bucket: mrapArn, Key });
    });
    it("should succeed with valid MRAP ARN", async () => {
      const result = await client.listObjects({
        Bucket: mrapArn,
      });
      expect(result.$metadata.httpStatusCode).toEqual(200);
      expect(result.Contents instanceof Array).toEqual(true);
    });
  });
});

export const createBuffer = (size: string) => {
  const KB_REGEX = /(\d+)KB/;
  const MB_REGEX = /(\d+)MB/;
  if (KB_REGEX.test(size)) {
    return new Uint8Array(parseInt(size.match(KB_REGEX)![1]) * 1024).fill(0x78);
  } else if (MB_REGEX.test(size)) {
    return new Uint8Array(
      parseInt(size.match(MB_REGEX)![1]) * 1024 * 1024
    ).fill(0x78);
  } else {
    return new Uint8Array(1024 * 1024).fill(0x78);
  }
};
