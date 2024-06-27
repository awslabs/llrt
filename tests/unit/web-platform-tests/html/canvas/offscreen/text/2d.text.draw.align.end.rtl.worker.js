// DO NOT EDIT! This test has been generated by /html/canvas/tools/gentest.py.
// OffscreenCanvas test in a worker:2d.text.draw.align.end.rtl
// Description:textAlign end with rtl is the left edge
// Note:

importScripts("/resources/testharness.js");
importScripts("/html/canvas/resources/canvas-tests.js");

promise_test(async t => {
  var canvas = new OffscreenCanvas(100, 50);
  var ctx = canvas.getContext('2d');

  var f = new FontFace("CanvasTest", "url('/fonts/CanvasTest.ttf')");
  f.load();
  self.fonts.add(f);
  await self.fonts.ready;
  ctx.font = '50px CanvasTest';
  ctx.direction = 'rtl';
  ctx.fillStyle = '#f00';
  ctx.fillRect(0, 0, 100, 50);
  ctx.fillStyle = '#0f0';
  ctx.textAlign = 'end';
  ctx.fillText('DD', 0, 37.5);
  _assertPixelApprox(canvas, 5,5, 0,255,0,255, 2);
  _assertPixelApprox(canvas, 95,5, 0,255,0,255, 2);
  _assertPixelApprox(canvas, 25,25, 0,255,0,255, 2);
  _assertPixelApprox(canvas, 75,25, 0,255,0,255, 2);
  _assertPixelApprox(canvas, 5,45, 0,255,0,255, 2);
  _assertPixelApprox(canvas, 95,45, 0,255,0,255, 2);
}, "textAlign end with rtl is the left edge");
done();
