// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

//! TLS Key Logging for debugging
//!
//! This module provides a `KeyLog` implementation that can emit key log events
//! for debugging TLS connections with tools like Wireshark.
//!
//! The key log format follows the NSS Key Log format:
//! `<label> <client_random_hex> <secret_hex>\n`

use std::fmt::Debug;
use std::sync::Arc;

use llrt_encoding::bytes_to_hex_string;
use rustls::KeyLog;
use tokio::sync::mpsc;

/// A key log line in NSS format
#[derive(Clone, Debug)]
pub struct KeyLogLine {
    pub line: String,
}

impl KeyLogLine {
    /// Format a key log line in NSS format
    pub fn new(label: &str, client_random: &[u8], secret: &[u8]) -> Self {
        let client_random_hex = bytes_to_hex_string(client_random);
        let secret_hex = bytes_to_hex_string(secret);
        Self {
            line: [label, " ", &client_random_hex, " ", &secret_hex, "\n"].concat(),
        }
    }

    /// Get the key log line as bytes
    pub fn as_bytes(&self) -> &[u8] {
        self.line.as_bytes()
    }
}

/// A KeyLog implementation that sends key log lines through a channel
pub struct ChannelKeyLog {
    sender: mpsc::UnboundedSender<KeyLogLine>,
}

impl Debug for ChannelKeyLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChannelKeyLog").finish()
    }
}

impl ChannelKeyLog {
    /// Create a new ChannelKeyLog and its receiver
    pub fn new() -> (Arc<Self>, mpsc::UnboundedReceiver<KeyLogLine>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        (Arc::new(Self { sender }), receiver)
    }
}

impl KeyLog for ChannelKeyLog {
    fn log(&self, label: &str, client_random: &[u8], secret: &[u8]) {
        let line = KeyLogLine::new(label, client_random, secret);
        // Ignore send errors - the receiver may have been dropped
        let _ = self.sender.send(line);
    }

    fn will_log(&self, _label: &str) -> bool {
        // We'll log all labels if there are listeners
        !self.sender.is_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keylog_line_format() {
        let line = KeyLogLine::new("CLIENT_RANDOM", &[0x01, 0x02, 0x03], &[0xaa, 0xbb, 0xcc]);
        assert_eq!(line.line, "CLIENT_RANDOM 010203 aabbcc\n");
    }

    #[tokio::test]
    async fn test_channel_keylog() {
        let (keylog, mut receiver) = ChannelKeyLog::new();

        keylog.log("TEST_LABEL", &[0x01, 0x02], &[0xaa, 0xbb]);

        let line = receiver.recv().await.unwrap();
        assert_eq!(line.line, "TEST_LABEL 0102 aabb\n");
    }
}
