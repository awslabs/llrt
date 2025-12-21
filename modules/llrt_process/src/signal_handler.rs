// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Unix signal handling for process.on('SIGTERM', ...) etc.
//!
//! Signal handlers are started lazily when the first listener is added for each signal.
//! Each Process instance maintains its own set of started signals, supporting multiple
//! runtimes correctly.
//!
//! Note: We check for signal events in each listener method (on, once, etc.) rather than
//! using `on_event_changed()` because:
//! 1. `on_event_changed()` only receives `&mut self`, not the `Ctx` needed to spawn async tasks
//! 2. Storing `Ctx` in Process causes lifetime issues (`Ctx` is not `'static`)
//! 3. Channel-based coordinator approaches cause issues with test harness shutdown

use std::collections::HashSet;

use llrt_context::CtxExtension;
use llrt_events::Emitter;
use rquickjs::{Class, Ctx, Persistent, Result};
use tokio::signal::unix::{signal, SignalKind};

use crate::Process;

/// Single source of truth for supported signals: (name, libc constant)
/// SignalKind is derived from the libc constant using SignalKind::from_raw()
static SUPPORTED_SIGNALS: &[(&str, libc::c_int)] = &[
    ("SIGTERM", libc::SIGTERM),
    ("SIGINT", libc::SIGINT),
    ("SIGHUP", libc::SIGHUP),
    ("SIGQUIT", libc::SIGQUIT),
    ("SIGUSR1", libc::SIGUSR1),
    ("SIGUSR2", libc::SIGUSR2),
];

/// Check if an event name corresponds to a supported Unix signal
pub fn is_signal_event(event: &str) -> bool {
    SUPPORTED_SIGNALS.iter().any(|(name, _)| *name == event)
}

/// Get SignalKind and signal number from signal name
fn get_signal_info(signal_name: &str) -> Option<(SignalKind, libc::c_int)> {
    SUPPORTED_SIGNALS
        .iter()
        .find(|(name, _)| *name == signal_name)
        .map(|(_, sig_num)| (SignalKind::from_raw(*sig_num), *sig_num))
}

/// Start a signal handler for a specific signal on this process instance.
pub fn start_signal_handler<'js>(
    ctx: &Ctx<'js>,
    process: &Class<'js, Process<'js>>,
    signal_name: &str,
    started_signals: &mut HashSet<String>,
) -> Result<()> {
    // Check if we already started a handler for this signal on this process instance
    if started_signals.contains(signal_name) {
        return Ok(());
    }

    let Some((signal_kind, sig_num)) = get_signal_info(signal_name) else {
        return Ok(());
    };

    // Mark as started for this process instance
    started_signals.insert(signal_name.to_string());

    let ctx = ctx.clone();
    let process = Persistent::save(&ctx, process.clone());
    let signal_name = signal_name.to_string();

    ctx.clone().spawn_exit_simple(async move {
        let mut sig = match signal(signal_kind) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    "Failed to register signal handler for {}: {}",
                    signal_name,
                    e
                );
                return Ok(());
            },
        };

        loop {
            sig.recv().await;

            let process = match process.clone().restore(&ctx) {
                Ok(p) => p,
                Err(_) => {
                    // Context is gone, stop the handler
                    break;
                },
            };

            // Check if there are listeners for this signal
            let has_listeners = process.borrow().has_listener_str(&signal_name);

            if has_listeners {
                tracing::trace!("Received signal {}, emitting event", signal_name);

                // Emit the signal event
                if let Err(e) = Process::emit_str(
                    rquickjs::prelude::This(process),
                    &ctx,
                    &signal_name,
                    vec![],
                    false,
                ) {
                    tracing::warn!("Error emitting signal {}: {:?}", signal_name, e);
                }
            } else {
                // No listeners - for SIGTERM/SIGINT, perform default action (exit)
                if signal_name == "SIGTERM" || signal_name == "SIGINT" {
                    tracing::trace!("Received {} with no listeners, exiting", signal_name);
                    std::process::exit(128 + sig_num);
                }
            }
        }

        Ok(())
    });

    Ok(())
}
