export async function splitStream(stream) {
  //stream is blob here
  const typedArray = await stream.bytes();
  return [typedArray.subarray(0, 3000), typedArray];
}
