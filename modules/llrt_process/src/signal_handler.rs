// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! Unix signal handling for process.on('SIGTERM', ...) etc.
//!
//! Uses a single coordinator task per Process instance that multiplexes all signal
//! streams using `tokio::select!`. Signals are registered lazily - only when a
//! listener is first added for that specific signal.
//!
//! Signal handling is triggered via the `on_event_changed()` callback in the Emitter trait,
//! which sends commands to the coordinator via an MPSC channel.

use std::collections::HashMap;

use llrt_context::CtxExtension;
use llrt_events::Emitter;
use rquickjs::{Class, Ctx, Persistent};
use tokio::signal::unix::{signal, Signal, SignalKind};
use tokio::sync::mpsc;

use crate::Process;

/// Single source of truth for supported signals: (name, libc constant)
/// SignalKind is derived from the libc constant using SignalKind::from_raw()
pub static SUPPORTED_SIGNALS: &[(&str, libc::c_int)] = &[
    ("SIGTERM", libc::SIGTERM),
    ("SIGINT", libc::SIGINT),
    ("SIGHUP", libc::SIGHUP),
    ("SIGQUIT", libc::SIGQUIT),
    ("SIGUSR1", libc::SIGUSR1),
    ("SIGUSR2", libc::SIGUSR2),
];

/// Commands sent to the signal coordinator
#[derive(Debug)]
pub enum SignalCommand {
    /// A listener was added for the specified signal
    AddListener(String),
    /// A listener was removed for the specified signal
    RemoveListener(String),
}

/// Check if an event name corresponds to a supported Unix signal
pub fn is_signal_event(event: &str) -> bool {
    SUPPORTED_SIGNALS.iter().any(|(name, _)| *name == event)
}

/// Helper that waits for a signal if registered, or pends forever if not.
/// This allows tokio::select! to effectively ignore unregistered signals.
async fn recv_if_registered(sig: &mut Option<Signal>) {
    match sig {
        Some(s) => {
            s.recv().await;
        },
        None => std::future::pending().await,
    }
}

/// Start the signal coordinator task for a Process instance.
///
/// Returns a channel sender that can be used to send commands to add/remove listeners.
/// The coordinator runs until the JS context is destroyed or the channel is closed.
///
/// Signals are registered lazily - only when the first listener is added for that signal.
/// This ensures we don't interfere with default OS behavior for signals the user never requested.
pub fn start_signal_coordinator<'js>(
    ctx: &Ctx<'js>,
    process: &Class<'js, Process<'js>>,
) -> mpsc::UnboundedSender<SignalCommand> {
    let (tx, mut rx) = mpsc::unbounded_channel();

    let ctx = ctx.clone();
    let process = Persistent::save(&ctx, process.clone());

    ctx.clone().spawn_exit_simple(async move {
        // Lazily initialized signals - None until first listener is added
        let mut sigterm: Option<Signal> = None;
        let mut sigint: Option<Signal> = None;
        let mut sighup: Option<Signal> = None;
        let mut sigquit: Option<Signal> = None;
        let mut sigusr1: Option<Signal> = None;
        let mut sigusr2: Option<Signal> = None;

        // Track listener counts per signal
        let mut listener_counts: HashMap<String, usize> = HashMap::new();

        // Macro to handle signal reception
        macro_rules! handle_signal {
            ($signal_name:expr, $sig_num:expr) => {{
                let count = listener_counts.get($signal_name).copied().unwrap_or(0);

                if count > 0 {
                    let process_instance = match process.clone().restore(&ctx) {
                        Ok(p) => p,
                        Err(_) => {
                            // Context is gone, stop coordinator
                            break;
                        },
                    };

                    tracing::trace!("Received signal {}, emitting event", $signal_name);

                    if let Err(e) = Process::emit_str(
                        rquickjs::prelude::This(process_instance),
                        &ctx,
                        $signal_name,
                        vec![],
                        false,
                    ) {
                        tracing::warn!("Error emitting signal {}: {:?}", $signal_name, e);
                    }
                } else {
                    // No listeners - for SIGTERM/SIGINT, perform default action (exit)
                    if $signal_name == "SIGTERM" || $signal_name == "SIGINT" {
                        tracing::trace!("Received {} with no listeners, exiting", $signal_name);
                        std::process::exit(128 + $sig_num);
                    }
                    // Other signals with no listeners are ignored
                }
            }};
        }

        // Macro to lazily register a signal
        macro_rules! register_signal {
            ($sig_var:ident, $sig_num:expr, $signal_name:expr) => {
                if $sig_var.is_none() {
                    match signal(SignalKind::from_raw($sig_num)) {
                        Ok(s) => {
                            tracing::trace!("Registered handler for {}", $signal_name);
                            $sig_var = Some(s);
                        },
                        Err(e) => {
                            tracing::warn!("Failed to register {} handler: {}", $signal_name, e);
                        },
                    }
                }
            };
        }

        loop {
            tokio::select! {
                _ = recv_if_registered(&mut sigterm) => {
                    handle_signal!("SIGTERM", libc::SIGTERM);
                }
                _ = recv_if_registered(&mut sigint) => {
                    handle_signal!("SIGINT", libc::SIGINT);
                }
                _ = recv_if_registered(&mut sighup) => {
                    handle_signal!("SIGHUP", libc::SIGHUP);
                }
                _ = recv_if_registered(&mut sigquit) => {
                    handle_signal!("SIGQUIT", libc::SIGQUIT);
                }
                _ = recv_if_registered(&mut sigusr1) => {
                    handle_signal!("SIGUSR1", libc::SIGUSR1);
                }
                _ = recv_if_registered(&mut sigusr2) => {
                    handle_signal!("SIGUSR2", libc::SIGUSR2);
                }
                cmd = rx.recv() => {
                    match cmd {
                        Some(SignalCommand::AddListener(sig)) => {
                            // Register signal lazily on first listener
                            match sig.as_str() {
                                "SIGTERM" => register_signal!(sigterm, libc::SIGTERM, "SIGTERM"),
                                "SIGINT" => register_signal!(sigint, libc::SIGINT, "SIGINT"),
                                "SIGHUP" => register_signal!(sighup, libc::SIGHUP, "SIGHUP"),
                                "SIGQUIT" => register_signal!(sigquit, libc::SIGQUIT, "SIGQUIT"),
                                "SIGUSR1" => register_signal!(sigusr1, libc::SIGUSR1, "SIGUSR1"),
                                "SIGUSR2" => register_signal!(sigusr2, libc::SIGUSR2, "SIGUSR2"),
                                _ => {},
                            }
                            *listener_counts.entry(sig).or_insert(0) += 1;
                        }
                        Some(SignalCommand::RemoveListener(sig)) => {
                            if let Some(count) = listener_counts.get_mut(&sig) {
                                *count = count.saturating_sub(1);
                            }
                        }
                        None => {
                            // Channel closed, exit coordinator
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    });

    tx
}
