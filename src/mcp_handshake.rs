// SPDX-FileCopyrightText: 2026 Fabio Rovai
// SPDX-License-Identifier: MIT

//! A guard over the MCP server transport before `initialize`.
//!
//! rmcp 1.4 ended the handshake when a client sent a non-ping request before
//! `initialize`. This wrapper answers out-of-order legacy requests instead of
//! closing stdio: unknown methods get -32601 and known methods get -32600.
//!
//! rmcp 3.9 recognizes `server/discover` as a bootstrap probe. Pass it to the
//! SDK for metadata validation and protocol negotiation, while keeping the
//! guard active until `initialize`. Discovery must not let a subsequent tool
//! request bypass initialization. The server advertises session-based protocol
//! revisions so HTTP requests retain their active ontology and lineage state.
//!
//! After `initialize`, every message, including notifications, reaches the SDK
//! unchanged. `tests/mcp_handshake_guard_test.rs` checks the legacy replies,
//! discovery failures, supported-version fallback and notification forwarding.

use rmcp::RoleServer;
use rmcp::model::{ClientJsonRpcMessage, ClientRequest, ErrorCode, ErrorData};
use rmcp::service::{RxJsonRpcMessage, TxJsonRpcMessage};
use rmcp::transport::{IntoTransport, Transport};

/// Answer out-of-order legacy requests without ending the stdio handshake,
/// while delegating discovery probes to the SDK.
pub fn guard<E, A>(
    transport: impl IntoTransport<RoleServer, E, A>,
) -> impl Transport<RoleServer, Error = E> + 'static
where
    E: std::error::Error + Send + Sync + 'static,
{
    HandshakeGuard {
        inner: transport.into_transport(),
        initialize_seen: false,
    }
}

struct HandshakeGuard<T> {
    inner: T,
    /// Set when the `initialize` request has been handed up to rmcp. From that
    /// point on this wrapper forwards every message without inspecting it.
    initialize_seen: bool,
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
            if self.initialize_seen {
                return Some(msg);
            }
            match &msg {
                ClientJsonRpcMessage::Request(req) => match &req.request {
                    ClientRequest::InitializeRequest(_) => {
                        self.initialize_seen = true;
                        return Some(msg);
                    }
                    // The SDK permits ping during its handshake.
                    ClientRequest::PingRequest(_) => return Some(msg),
                    // Since rmcp 3.9, `server/discover` is the 2026-07-28
                    // protocol's bootstrap probe, which a client sends before
                    // `initialize` by design. rmcp answers it in its own
                    // handshake loop and stays open for `initialize` after:
                    // with the version list when the probe names a revision
                    // this server supports, otherwise with an error naming
                    // what is missing or supported. Intercepting it here
                    // would hide that answer behind an ordering fault.
                    ClientRequest::DiscoverRequest(_) => return Some(msg),
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
