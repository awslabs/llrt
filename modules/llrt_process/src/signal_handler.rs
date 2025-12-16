// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Unix signal handling for process.on('SIGTERM', ...) etc.

use std::sync::atomic::{AtomicBool, Ordering};

use llrt_context::CtxExtension;
use llrt_events::Emitter;
use rquickjs::{Class, Ctx, Persistent, Result};
use tokio::signal::unix::{signal, SignalKind};

use crate::Process;

/// Flags to track which signal handlers have been started
static SIGTERM_HANDLER_STARTED: AtomicBool = AtomicBool::new(false);
static SIGINT_HANDLER_STARTED: AtomicBool = AtomicBool::new(false);
static SIGHUP_HANDLER_STARTED: AtomicBool = AtomicBool::new(false);
static SIGQUIT_HANDLER_STARTED: AtomicBool = AtomicBool::new(false);
static SIGUSR1_HANDLER_STARTED: AtomicBool = AtomicBool::new(false);
static SIGUSR2_HANDLER_STARTED: AtomicBool = AtomicBool::new(false);

/// Check if an event name corresponds to a Unix signal
pub fn is_signal_event(event: &str) -> bool {
    matches!(
        event,
        "SIGTERM" | "SIGINT" | "SIGHUP" | "SIGQUIT" | "SIGUSR1" | "SIGUSR2"
    )
}

/// Get signal kind and started flag for a signal name
fn get_signal_info(signal_name: &str) -> Option<(SignalKind, &'static AtomicBool)> {
    match signal_name {
        "SIGTERM" => Some((SignalKind::terminate(), &SIGTERM_HANDLER_STARTED)),
        "SIGINT" => Some((SignalKind::interrupt(), &SIGINT_HANDLER_STARTED)),
        "SIGHUP" => Some((SignalKind::hangup(), &SIGHUP_HANDLER_STARTED)),
        "SIGQUIT" => Some((SignalKind::quit(), &SIGQUIT_HANDLER_STARTED)),
        "SIGUSR1" => Some((SignalKind::user_defined1(), &SIGUSR1_HANDLER_STARTED)),
        "SIGUSR2" => Some((SignalKind::user_defined2(), &SIGUSR2_HANDLER_STARTED)),
        _ => None,
    }
}

/// Start a signal handler for a specific signal when a listener is first registered
pub fn maybe_start_signal_handler<'js>(
    ctx: &Ctx<'js>,
    process: &Class<'js, Process<'js>>,
    signal_name: &str,
) -> Result<()> {
    let Some((signal_kind, started_flag)) = get_signal_info(signal_name) else {
        return Ok(());
    };

    // Only start the handler once
    if started_flag.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

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
                    std::process::exit(128 + get_signal_number(&signal_name));
                }
            }
        }

        Ok(())
    });

    Ok(())
}

/// Get the numeric signal value for exit code calculation
fn get_signal_number(signal_name: &str) -> i32 {
    match signal_name {
        "SIGHUP" => 1,
        "SIGINT" => 2,
        "SIGQUIT" => 3,
        "SIGTERM" => 15,
        "SIGUSR1" => 10,
        "SIGUSR2" => 12,
        _ => 0,
    }
}
