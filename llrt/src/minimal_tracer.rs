// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use std::{
    env,
    fmt::{self, Write},
    sync::atomic::{AtomicUsize, Ordering},
    write,
};

use chrono::{DateTime, Utc};
use tracing::{field::Visit, Id, Level, Subscriber};
use tracing_core::{span, Field};

use llrt_core::environment;

pub struct StringVisitor<'a> {
    string: &'a mut String,
}
impl<'a> StringVisitor<'a> {
    pub(crate) fn new(string: &'a mut String) -> Self {
        StringVisitor { string }
    }
}

impl Visit for StringVisitor<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            write!(self.string, "{value:?} ").unwrap();
        } else {
            write!(self.string, "{} = {:?}; ", field.name(), value).unwrap();
        }
    }
}

struct LogFilter {
    target: Option<String>,
    level: Option<Level>,
}

pub struct MinimalTracer {
    enabled: bool,
    filters: Vec<LogFilter>,
}

fn string_to_level(string: &str) -> Option<Level> {
    match string.to_lowercase().as_str() {
        "info" => Some(Level::INFO),
        "debug" => Some(Level::DEBUG),
        "warn" | "warning" => Some(Level::WARN),
        "trace" => Some(Level::TRACE),
        "error" => Some(Level::ERROR),
        _ => None,
    }
}

impl MinimalTracer {
    pub fn register() -> Result<(), tracing::subscriber::SetGlobalDefaultError> {
        let mut enabled = false;
        let mut filters: Vec<LogFilter> = Vec::with_capacity(10);
        if let Ok(env_value) = env::var(environment::ENV_LLRT_LOG) {
            enabled = true;
            for filter in env_value.split(',') {
                let mut target = Some(filter);
                let mut level = None;
                if let Some(equals_index) = target.unwrap().find('=') {
                    let (first, second) = filter.split_at(equals_index);
                    target = Some(first);
                    level = string_to_level(&second[1..])
                }
                let target_level = string_to_level(target.unwrap());

                if let Some(target_level) = target_level {
                    level = Some(target_level);
                    target = None;
                }

                filters.push(LogFilter {
                    target: target.map(|v| v.to_string()),
                    level,
                });
            }
        }

        tracing::subscriber::set_global_default(MinimalTracer { enabled, filters })
    }
}

static AUTO_ID: AtomicUsize = AtomicUsize::new(1);

impl Subscriber for MinimalTracer {
    fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
        if self.enabled {
            if self.filters.is_empty() {
                return true;
            }

            let mut matches: bool;
            for filter in &self.filters {
                matches = true;
                if let Some(level) = filter.level {
                    if metadata.level() > &level {
                        matches = false;
                    }
                }
                if let Some(target) = &filter.target {
                    if !metadata.target().starts_with(target) {
                        matches = false;
                    }
                }
                if matches {
                    return true;
                }
            }
            return false;
        }
        false
    }

    fn new_span(&self, _span: &span::Attributes<'_>) -> span::Id {
        Id::from_u64(AUTO_ID.fetch_add(1, Ordering::Relaxed) as u64)
    }

    fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

    fn event(&self, event: &tracing::Event<'_>) {
        let metadata = event.metadata();

        let level = metadata.level();
        let target = metadata.target();

        let mut text = String::new();

        let mut visitor = StringVisitor::new(&mut text);
        event.record(&mut visitor);

        let current_time: DateTime<Utc> = Utc::now();
        let timestamp = current_time.format("%Y-%m-%dT%H:%M:%S%.3fZ");

        println!("{timestamp} {level} {target}: {text}");
    }

    fn enter(&self, _span: &span::Id) {}

    fn exit(&self, _span: &span::Id) {}
}
