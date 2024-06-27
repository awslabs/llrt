// DO NOT EDIT! This test has been generated by /html/canvas/tools/gentest.py.
// OffscreenCanvas test in a worker:2d.drawImage.clip
// Description:
// Note:

importScripts("/resources/testharness.js");
importScripts("/html/canvas/resources/canvas-tests.js");

promise_test(async t => {
  var canvas = new OffscreenCanvas(100, 50);
  var ctx = canvas.getContext('2d');

  ctx.fillStyle = '#0f0';
  ctx.fillRect(0, 0, 100, 50);
  ctx.rect(-10, -10, 1, 1);
  ctx.clip();
  const response = await fetch('/images/red.png');
  const blob = await response.blob();
  const bitmap = await createImageBitmap(blob);

  ctx.fillStyle = '#0f0';
  ctx.fillRect(0, 0, 100, 50);
  ctx.rect(-10, -10, 1, 1);
  ctx.clip();
  ctx.drawImage(bitmap, 0, 0);
  _assertPixelApprox(canvas, 50,25, 0,255,0,255, 2);
}, "");
done();
