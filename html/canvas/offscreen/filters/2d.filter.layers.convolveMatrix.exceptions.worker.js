// DO NOT EDIT! This test has been generated by /html/canvas/tools/gentest.py.
// OffscreenCanvas test in a worker:2d.filter.layers.convolveMatrix.exceptions
// Description:Test exceptions on CanvasFilter() convolveMatrix
// Note:

importScripts("/resources/testharness.js");
importScripts("/html/canvas/resources/canvas-tests.js");

var t = async_test("Test exceptions on CanvasFilter() convolveMatrix");
var t_pass = t.done.bind(t);
var t_fail = t.step_func(function(reason) {
    throw reason;
});
t.step(function() {

  var canvas = new OffscreenCanvas(100, 50);
  var ctx = canvas.getContext('2d');

  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix'}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', divisor: 2}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: null}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: 1}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [[1, 0], [0]]}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [[1, 'a'], [0]]}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [[1, 0], 0]}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [[1, 0], [0, Infinity]]}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: []}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [1]}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [1, 2, 3, 4]}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [[], []]}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [[1, 2], []]}}); });
  assert_throws_js(TypeError, function() { ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [[], [1, 2]]}}); });
  // This should not throw an error
  ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [[]]}});
  ctx.endLayer();
  ctx.beginLayer({filter:
    {name: 'convolveMatrix', kernelMatrix: [[1]]}});
  ctx.endLayer();
  t.done();
});
done();
