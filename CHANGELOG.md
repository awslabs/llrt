* Over 2x faster JSON parsing & stringify:
  
        Size 2580:
                parse: 49.333µs vs. 89.792µs
                stringify: 31µs vs. 92.375µs
        Size 51701:
                parse: 494.458µs vs. 1.160125ms
                stringify: 427.791µs vs. 1.097625ms
        Size 517001:
                parse: 4.932875ms vs. 11.911375ms
                stringify: 3.925875ms vs. 10.853125ms
        Size 5170901:
                parse: 56.0855ms vs. 126.783833ms
                stringify: 38.671083ms vs. 107.312875ms
        Size 51718901:
                parse: 731.7205ms vs. 1.285825541s
                stringify: 395.82225ms vs. 1.39267225s
        Size 517288901:
                parse: 6.886183416s vs. 14.985707583s
                stringify: 3.957781167s vs. 10.885577917s

* 7x faster integer and float toString():
        
        Benchmark 1: target/release/llrt
        Time (mean ± σ):      1.568 s ±  0.016 s    [User: 1.555 s, System: 0.007 s]
        Range (min … max):    1.558 s …  1.610 s    10 runs
        
        Benchmark 2: target/release/llrt-next
        Time (mean ± σ):     205.1 ms ±   3.1 ms    [User: 196.9 ms, System: 2.2 ms]
        Range (min … max):   200.0 ms … 213.1 ms    14 runs
        
        Summary
        target/release/llrt-next ran
        7.65 ± 0.14 times faster than target/release/llrt

* Improved logging:
  * LLRT now supports [advanced logging controls](https://aws.amazon.com/blogs/compute/introducing-advanced-logging-controls-for-aws-lambda-functions/) for AWS Lambda
  * `requestId` is now captured and outputted with logging
  * Console has some performance improvements by reusing String and avoiding allocations
