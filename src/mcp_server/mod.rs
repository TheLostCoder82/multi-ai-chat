//! MCP Server Module
//! 
//! This module implements the Model Context Protocol (MCP) server
//! that allows AI agents to connect as clients.

mod server;
mod session;
mod protocol;
mod handshake;

pub use server::McpServer;
pub use session::Session;
pub use protocol::{McpMessage, McpRequest, McpResponse, McpMethod};
pub use handshake::HandshakeState;
