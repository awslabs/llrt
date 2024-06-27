// DO NOT EDIT! This test has been generated by /html/canvas/tools/gentest.py.
// OffscreenCanvas test in a worker:2d.drawImage.floatsource
// Description:
// Note:

importScripts("/resources/testharness.js");
importScripts("/html/canvas/resources/canvas-tests.js");

promise_test(async t => {
  var canvas = new OffscreenCanvas(100, 50);
  var ctx = canvas.getContext('2d');

  const response = await fetch('/images/green.png');
  const blob = await response.blob();
  const bitmap = await createImageBitmap(blob);
  ctx.drawImage(bitmap, 10.1, 10.1, 0.1, 0.1, 0, 0, 100, 50);
  _assertPixelApprox(canvas, 50,25, 0,255,0,255, 2);
}, "");
done();
