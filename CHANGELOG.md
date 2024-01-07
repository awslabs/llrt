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
