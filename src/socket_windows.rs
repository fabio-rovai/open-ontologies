// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use crate::graph::GraphStore;

/// Windows stub for the Unix domain socket adapter.
pub async fn serve(_socket_path: &str, _graph: Arc<GraphStore>) -> anyhow::Result<()> {
    anyhow::bail!(
        "serve-unix is not available on Windows yet. Use `serve` or `serve-http` instead."
    );
}
