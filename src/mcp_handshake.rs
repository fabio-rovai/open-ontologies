//! A guard over the MCP server transport for the window before `initialize`.
//!
//! rmcp's server handshake reads messages itself until it sees `initialize`
//! (rmcp-1.4.0/src/service/server.rs:170-203). It answers a pre-initialize
//! `ping` in that loop and it accepts nothing else: any other request falls
//! through `ClientJsonRpcMessage::Request(req) => break (req.request, req.id)`
//! at line 191 into the `else` at line 200, and `serve` returns
//! `ServerInitializeError::ExpectedInitializeRequest`. `main` propagates that
//! with `?` at main.rs:1867, the process exits, and the client sees EOF.
//!
//! Issue #260 is that failure caused by one line. A client sent
//! `{"jsonrpc":"2.0","id":1,"method":"server/discover","params":{}}` as its
//! first message and 2.0 died before answering the handshake at all. The same
//! request one message later, after `initialize`, is answered -32601 by rmcp's
//! default `on_custom_request` (rmcp-1.4.0/src/handler/server.rs:282-294) and
//! costs nothing. Arrival order was the only difference between a reply and a
//! dead process.
//!
//! Answering instead of exiting is a deliberate protocol choice. JSON-RPC 2.0
//! requires a reply to every request that carries an id, and the MCP lifecycle
//! says a client SHOULD NOT send non-ping requests before the initialize
//! response, not MUST NOT. Exiting without a reply is the less conformant of
//! the two behaviours, so this wrapper replies. It changes nothing after the
//! handshake: once the `initialize` request has passed through, every message
//! reaches rmcp untouched, which is what
//! `guard_is_transparent_after_initialize` in
//! `tests/mcp_handshake_guard_test.rs` measures.
//!
//! rmcp's own authors reason the same way one screen further down. The comment
//! at rmcp-1.4.0/src/service/server.rs:250-256 says the spec uses SHOULD NOT
//! rather than MUST NOT for pre-`initialized` messages and that a request
//! arriving before the `initialized` notification is therefore processed
//! normally. This wrapper applies their stated rule one message earlier.

use rmcp::RoleServer;
use rmcp::model::{ClientJsonRpcMessage, ClientRequest, ErrorCode, ErrorData};
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
                    // rmcp answers a pre-initialize ping itself, in the
                    // same loop that rejects everything else, so this one
                    // still goes up.
                    ClientRequest::PingRequest(_) => return Some(msg),
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
                            .send(TxJsonRpcMessage::<RoleServer>::error(error, id))
                            .await
                        {
                            tracing::warn!("could not answer a pre-initialize request: {e}");
                        }
                    }
                },
                // Nothing without an id is owed a reply, so a stray
                // pre-initialize notification or response is dropped rather
                // than ending the handshake the way rmcp's `other =>` arm
                // at server.rs:192-196 does.
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
