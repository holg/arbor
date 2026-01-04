//! Arbor Server - WebSocket server for the Arbor Protocol
//!
//! This crate implements the server side of the Arbor Protocol,
//! allowing AI agents and IDE integrations to query the code graph.
//!
//! The server supports:
//! - Multiple concurrent connections
//! - JSON-RPC 2.0 messages
//! - Real-time graph updates via subscriptions

mod handlers;
mod protocol;
mod server;

pub use protocol::{Request, Response, RpcError};
pub use server::{ArborServer, ServerConfig};
