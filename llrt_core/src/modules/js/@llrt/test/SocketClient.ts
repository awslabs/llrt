import { Socket } from "node:net";
import { EventEmitter } from "node:events";

class SocketClient extends EventEmitter {
  private host: string;
  private port: number;
  private socket: Socket;
  private queue: Array<{
    data: string;
    resolve: (value: Buffer) => void;
    reject: (reason?: any) => void;
  }>;
  private currentlySending: boolean;
  private currentResolve?: (value: Buffer) => void;

  constructor(host: string, port: number) {
    super();
    this.host = host;
    this.port = port;
    this.socket = new Socket();
    this.queue = [];
    this.currentlySending = false;

    this.socket.on("data", (data) => this.handleResponse(data));

    this.socket.on("close", () => {
      return this.emit("close");
    });
  }

  public async connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      const errorListener = (err: Error) => reject(err);
      this.socket.on("error", errorListener);
      this.socket.connect(this.port, this.host, () => {
        this.socket.off("error", errorListener);
        this.socket.on("error", (err) => this.emit("error", err));
        resolve();
      });
    });
  }

  public async send(data: string): Promise<Buffer> {
    return new Promise<Buffer>((resolve, reject) => {
      this.queue.push({ data, resolve, reject });
      this.processQueue();
    });
  }

  private async processQueue(): Promise<void> {
    if (this.currentlySending || this.queue.length === 0) return;

    const { data, resolve, reject } = this.queue.shift()!;
    this.currentlySending = true;

    try {
      this.currentResolve = resolve;
      await this.sendData(data);
    } catch (err) {
      reject(err);
      this.currentlySending = false;
      this.processQueue();
    }
  }

  private async sendData(data: string): Promise<void> {
    return new Promise((resolve, reject) => {
      this.socket.write(data, (err) => {
        if (err) {
          return reject(err);
        }
        resolve();
      });
    });
  }

  private handleResponse(data: Buffer): void {
    if (this.currentResolve) {
      this.currentResolve(data);
      this.currentlySending = false;
      this.processQueue();
    }
  }

  async close(): Promise<void> {
    return new Promise((resolve) => {
      this.socket.end(resolve);
    });
  }
}

export default SocketClient;
