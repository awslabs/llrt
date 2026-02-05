use std::collections::VecDeque;

use rquickjs::{class::Trace, Ctx, Exception, JsLifetime, Result, Value};

use crate::queuing_strategy::SizeValue;

/// QueueWithSize is present in readable and writable streams and abstracts away certain queue operations
/// https://streams.spec.whatwg.org/#queue-with-sizes
#[derive(JsLifetime, Trace, Default)]
pub struct QueueWithSizes<'js> {
    pub queue: VecDeque<ValueWithSize<'js>>,
    pub queue_total_size: f64,
}

impl<'js> QueueWithSizes<'js> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
            queue_total_size: 0.0,
        }
    }

    pub fn enqueue_value_with_size(
        &mut self,
        ctx: &Ctx<'js>,
        value: Value<'js>,
        size: SizeValue<'js>,
    ) -> Result<()> {
        let size = match is_non_negative_number(size) {
            None => {
                // If ! IsNonNegativeNumber(size) is false, throw a RangeError exception.
                return Err(Exception::throw_range(
                    ctx,
                    "Size must be a finite, non-NaN, non-negative number.",
                ));
            },
            Some(size) => size,
        };

        // If size is +∞, throw a RangeError exception.
        if size.is_infinite() {
            return Err(Exception::throw_range(
                ctx,
                "Size must be a finite, non-NaN, non-negative number.",
            ));
        };

        // Append a new value-with-size with value value and size size to container.[[queue]].
        self.queue.push_back(ValueWithSize { value, size });

        // Set container.[[queueTotalSize]] to container.[[queueTotalSize]] + size.
        self.queue_total_size += size;

        Ok(())
    }

    pub fn dequeue_value(&mut self) -> Value<'js> {
        // Let valueWithSize be container.[[queue]][0].
        // Remove valueWithSize from container.[[queue]].
        let value_with_size = self
            .queue
            .pop_front()
            .expect("DequeueValue called with empty queue");
        // Set container.[[queueTotalSize]] to container.[[queueTotalSize]] − valueWithSize’s size.
        self.queue_total_size -= value_with_size.size;
        // If container.[[queueTotalSize]] < 0, set container.[[queueTotalSize]] to 0. (This can occur due to rounding errors.)
        if self.queue_total_size < 0.0 {
            self.queue_total_size = 0.0
        }
        value_with_size.value
    }

    pub fn reset_queue(&mut self) {
        // Set container.[[queue]] to a new empty list.
        self.queue.clear();
        // Set container.[[queueTotalSize]] to 0.
        self.queue_total_size = 0.0;
    }
}

#[derive(JsLifetime, Trace, Clone)]
pub struct ValueWithSize<'js> {
    pub value: Value<'js>,
    size: f64,
}

fn is_non_negative_number(value: SizeValue<'_>) -> Option<f64> {
    // If Type(v) is not Number, return false.
    let number = value.as_number()?;
    // If v is NaN, return false.
    if number.is_nan() {
        return None;
    }

    // If v < 0, return false.
    if number < 0.0 {
        return None;
    }

    // Return true.
    Some(number)
}
