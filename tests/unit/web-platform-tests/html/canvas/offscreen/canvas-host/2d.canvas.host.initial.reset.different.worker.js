// DO NOT EDIT! This test has been generated by /html/canvas/tools/gentest.py.
// OffscreenCanvas test in a worker:2d.canvas.host.initial.reset.different
// Description:Changing size resets canvas to transparent black
// Note:

importScripts("/resources/testharness.js");
importScripts("/html/canvas/resources/canvas-tests.js");

var t = async_test("Changing size resets canvas to transparent black");
var t_pass = t.done.bind(t);
var t_fail = t.step_func(function(reason) {
    throw reason;
});
t.step(function() {

  var canvas = new OffscreenCanvas(100, 50);
  var ctx = canvas.getContext('2d');

  ctx.fillStyle = '#f00';
  ctx.fillRect(0, 0, 50, 50);
  _assertPixel(canvas, 20,20, 255,0,0,255);
  canvas.width = 50;
  _assertPixel(canvas, 20,20, 0,0,0,0);
  t.done();
});
done();
