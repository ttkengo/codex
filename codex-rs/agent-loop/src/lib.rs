//! # Codex Agent Loop
//!
//! A simplified wrapper library around `codex-core` that provides an easy-to-use
//! API for building AI agents with LLM interaction, tool execution, MCP support,
//! and conversation history management.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use codex_agent_loop::{AgentBuilder, SimpleAgent};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let agent = AgentBuilder::new()
//!         .with_api_key("your-api-key")
//!         .with_model("gpt-4")
//!         .with_workspace("./my-project")
//!         .build()
//!         .await?;
//!    
//!     let response = agent
//!         .send_message("Write a hello world program in Rust")
//!         .await?;
//!    
//!     println!("Agent: {}", response);
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! This crate wraps `codex-core` without modifying any of its code, making it
//! easy to sync with upstream Codex changes. It provides:
//!
//! - **SimpleAgent**: High-level wrapper for common agent use cases
//! - **AgentBuilder**: Fluent API for configuring agents
//! - **Helper functions**: Convenient methods for message passing and event handling
//! - **Extension traits**: Optional traits for custom LLM providers and tools

// Prevent direct stdout/stderr writes
#![deny(clippy::print_stdout, clippy::print_stderr)]

// Module declarations
mod adapters;
mod builder;
mod helpers;
mod simple;
mod traits;

// Re-export main types
pub use builder::AgentBuilder;
pub use helpers::ask_once;
pub use helpers::batch_ask;
pub use helpers::interactive_session;
pub use simple::SimpleAgent;
pub use traits::LlmProvider;
pub use traits::ToolExtension;

// Re-export commonly used types from codex-core and codex-protocol
pub use codex_core::AuthManager;
pub use codex_core::CodexAuth;
pub use codex_core::CodexThread;
pub use codex_core::ThreadManager;
pub use codex_core::config::Config;

pub use codex_protocol::ThreadId;
pub use codex_protocol::protocol::Event;
pub use codex_protocol::protocol::EventMsg;
pub use codex_protocol::protocol::Op;
pub use codex_protocol::protocol::Submission;
pub use codex_protocol::user_input::UserInput;

// Re-export error types
pub use codex_core::error::CodexErr;
pub use codex_core::error::Result as CodexResult;

// Module exports for advanced usage
pub mod event_filter {
    //! Utilities for filtering and processing agent events.
    pub use crate::adapters::EventFilter;
    pub use crate::adapters::FilteredEvent;
}

pub mod op_builder {
    //! Builder utilities for constructing Codex operations.
    pub use crate::adapters::OpBuilder;
}

/// Module for batch helper utilities.
pub mod batch {
    pub use crate::helpers::batch_ask;
}

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::AgentBuilder;
    pub use crate::SimpleAgent;
    pub use crate::ask_once;
    pub use crate::batch_ask;
    pub use crate::interactive_session;
    pub use codex_protocol::protocol::Event;
    pub use codex_protocol::protocol::EventMsg;
    pub use codex_protocol::protocol::Op;
}
