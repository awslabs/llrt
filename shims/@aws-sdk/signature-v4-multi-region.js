import { SignatureV4S3Express } from "@aws-sdk/middleware-sdk-s3";
import { SignatureV4a } from "@smithy/signature-v4a";

export const signatureV4CrtContainer = { CrtSignerV4: null };

export class SignatureV4MultiRegion {
  constructor(options) {
    this.sigv4Signer = new SignatureV4S3Express(options);
    this.signerOptions = options;
  }

  static sigv4aDependency() {
    return "js";
  }

  async sign(requestToSign, options = {}) {
    if (options.signingRegion === "*") {
      return this._getSigv4aSigner().sign(requestToSign, options);
    }
    return this.sigv4Signer.sign(requestToSign, options);
  }

  async signWithCredentials(requestToSign, credentials, options = {}) {
    return this.sigv4Signer.signWithCredentials(
      requestToSign,
      credentials,
      options
    );
  }

  async presign(originalRequest, options = {}) {
    return this.sigv4Signer.presign(originalRequest, options);
  }

  async presignWithCredentials(originalRequest, credentials, options = {}) {
    return this.sigv4Signer.presignWithCredentials(
      originalRequest,
      credentials,
      options
    );
  }

  _getSigv4aSigner() {
    if (!this.sigv4aSigner) {
      this.sigv4aSigner = new SignatureV4a({ ...this.signerOptions });
    }
    return this.sigv4aSigner;
  }

  // Keep backward compat with SDK internals
  getSigv4aSigner() {
    return this._getSigv4aSigner();
  }
}
