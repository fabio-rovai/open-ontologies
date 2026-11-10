// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! A guard over the MCP server transport for the window before `initialize`.
//!
//! In rmcp 1.4, the handshake answered `ping` but rejected every other request
//! before `initialize`, returning `ExpectedInitializeRequest`. Issue #260 was
//! a client's `server/discover` probe ending the process before initialization.
//! This wrapper preserves the JSON-RPC error replies introduced for that bug.
//!
//! rmcp 3.9 also supports discovery and inline protocol negotiation without
//! `initialize`. Requests carrying protocol-version metadata reach the SDK
//! unchanged so it can validate and negotiate them, then owns the rest of the
//! lifecycle. Legacy clients still receive the guard's established error replies.
//!
//! Answering instead of exiting is a deliberate protocol choice. JSON-RPC 2.0
//! requires a reply to every request that carries an id, and the MCP lifecycle
//! says a client SHOULD NOT send non-ping requests before the initialize
//! response, not MUST NOT. Exiting without a reply is the less conformant of
//! the two behaviours, so this wrapper replies. It changes nothing after the
//! handshake: once `initialize` or inline negotiation has passed through, every
//! message reaches rmcp untouched, which is what
//! `guard_is_transparent_after_initialize` in
//! `tests/mcp_handshake_guard_test.rs` measures.
//!
use rmcp::RoleServer;
use rmcp::model::{ClientJsonRpcMessage, ClientRequest, ErrorCode, ErrorData, GetMeta};
use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::{IntoTransport, Transport};

/// Wrap a transport so that a request arriving before `initialize` is answered
/// with a JSON-RPC error instead of ending the process.
pub fn guard<E, A>(
    transport: impl IntoTransport<RoleServer, E, A>,
) -> impl Transport<RoleServer, Error = E> + 'static
where
    E: std::error::Error + Send + Sync + 'static,
{
    HandshakeGuard {
        inner: transport.into_transport(),
        sdk_lifecycle_seen: false,
    }
}

struct HandshakeGuard<T> {
    inner: T,
    /// Set when initialize or inline negotiation has been handed to rmcp.
    /// From that point on the SDK owns validation of every message, including
    /// notifications and retries after incomplete inline metadata.
    sdk_lifecycle_seen: bool,
}

impl<T> Transport<RoleServer> for HandshakeGuard<T>
where
    T: Transport<RoleServer> + Send + 'static,
{
    type Error = T::Error;

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleServer>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send + 'static {
        self.inner.send(item)
    }

    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleServer>> {
        loop {
            let msg = self.inner.receive().await?;
            if self.sdk_lifecycle_seen {
                return Some(msg);
            }
            match &msg {
                ClientJsonRpcMessage::Request(req) => match &req.request {
                    ClientRequest::InitializeRequest(_) => {
                        self.sdk_lifecycle_seen = true;
                        return Some(msg);
                    }
                    // rmcp answers a pre-initialize ping itself, in the
                    // same loop that rejects everything else, so this one
                    // still goes up.
                    ClientRequest::PingRequest(_) => return Some(msg),
                    // The SDK validates inline metadata and supports discovery
                    // before initialize. Do not mistake that lifecycle for an
                    // out-of-order request from a legacy client.
                    other if other.get_meta().protocol_version().is_some() => {
                        self.sdk_lifecycle_seen = true;
                        return Some(msg);
                    }
                    other => {
                        let method = other.method().to_string();
                        let id = req.id.clone();
                        // An unknown method gets the code rmcp's own
                        // `on_custom_request` gives it after the
                        // handshake, so the reply a client sees does not
                        // depend on which side of `initialize` its probe
                        // landed on. A known MCP method sent early is a
                        // real ordering fault and is named as one.
                        let error = if matches!(other, ClientRequest::CustomRequest(_)) {
                            ErrorData::new(ErrorCode::METHOD_NOT_FOUND, method.clone(), None)
                        } else {
                            ErrorData::new(
                                ErrorCode::INVALID_REQUEST,
                                format!("{method} was received before initialize"),
                                None,
                            )
                        };
                        tracing::info!(
                            "answering {method} with an error because it arrived before initialize"
                        );
                        if let Err(e) = self
                            .inner
                            .send(TxJsonRpcMessage::<RoleServer>::error(error, Some(id)))
                            .await
                        {
                            tracing::warn!("could not answer a pre-initialize request: {e}");
                        }
                    }
                },
                // Nothing without an id is owed a reply, so a stray
                // pre-initialize notification or response is dropped rather
                // than ending the SDK's legacy handshake.
                _ => {
                    tracing::debug!("dropping a message received before initialize: {msg:?}");
                }
            }
        }
    }

    fn close(&mut self) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        self.inner.close()
    }
}
