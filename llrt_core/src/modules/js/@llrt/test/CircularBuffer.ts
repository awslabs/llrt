export default class CircularBuffer {
  private buffer: Buffer;
  private maxSize: number;
  private currentPosition: number;
  private isFull: boolean;
  private atCapacity: boolean;

  constructor(maxSize: number, initialSize: number = maxSize / 8) {
    this.maxSize = Math.ceil(maxSize / 8) * 8;
    const adjustedInitialSize = Math.ceil(initialSize / 8) * 8;
    this.buffer = Buffer.alloc(adjustedInitialSize);
    this.currentPosition = 0;
    this.isFull = false;
    this.atCapacity = false;
  }

  private grow(targetSize: number): void {
    const newSize = Math.min(
      Math.pow(2, Math.ceil(Math.log2(targetSize))),
      this.maxSize
    );

    if (newSize === this.maxSize) {
      this.atCapacity = true;
    }

    const newBuffer = Buffer.alloc(newSize);

    newBuffer.set(this.getContent());
    this.buffer = newBuffer;
    this.isFull = false;
  }

  clear() {
    this.currentPosition = 0;
    this.isFull = false;
  }

  append(data: Uint8Array): void {
    //if data is larger than maxSize, just keep the last maxSize bytes
    if (data.length >= this.maxSize) {
      //we are not at max size yet
      if (!this.atCapacity) {
        this.buffer = Buffer.alloc(this.maxSize);
      }
      //copy over last bytes to fill buffer
      this.buffer.set(data.slice(-this.maxSize));
      this.currentPosition = 0;
      this.isFull = true;
      return;
    }

    if (
      !this.atCapacity &&
      this.currentPosition + data.length > this.buffer.length
    ) {
      this.grow(this.currentPosition + data.length);
    }

    //wrap around
    if (this.currentPosition + data.length >= this.buffer.length) {
      const firstPart = this.buffer.length - this.currentPosition;
      this.buffer.set(data.subarray(0, firstPart), this.currentPosition);
      this.buffer.set(data.subarray(firstPart));
      this.currentPosition = data.length - firstPart;
      this.isFull = true;
    } else {
      this.buffer.set(data, this.currentPosition);
      this.currentPosition += data.length;
    }
  }

  getContent(): Buffer {
    if (!this.isFull) {
      return this.buffer.subarray(0, this.currentPosition);
    }

    return Buffer.concat([
      this.buffer.subarray(this.currentPosition),
      this.buffer.subarray(0, this.currentPosition),
    ]);
  }
}
