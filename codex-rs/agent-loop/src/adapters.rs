//! Adapters for converting between generic agent types and Codex internal types.

use codex_protocol::protocol::Event;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::user_input::UserInput;

/// Filter and simplify Codex events for common use cases.
///
/// This utility helps process the raw event stream from Codex and extract
/// the most relevant information for common use cases.
#[derive(Default)]
pub struct EventFilter;

impl EventFilter {
    /// Create a new event filter.
    pub fn new() -> Self {
        Self
    }

    /// Filter an event to a simplified representation.
    ///
    /// Returns `None` for events that are typically ignored in simple use cases
    /// (like progress updates, debug events, etc.).
    pub fn filter(&self, event: &Event) -> Option<FilteredEvent> {
        match &event.msg {
            EventMsg::AgentMessageContentDelta(delta) => {
                Some(FilteredEvent::MessageDelta(delta.delta.clone()))
            }
            EventMsg::TurnComplete(complete) => Some(FilteredEvent::TurnComplete {
                last_message: complete.last_agent_message.clone(),
            }),
            EventMsg::TurnAborted(abort) => Some(FilteredEvent::TurnAborted {
                reason: format!("{reason:?}", reason = abort.reason),
            }),
            EventMsg::Error(err) => Some(FilteredEvent::Error(err.message.clone())),
            EventMsg::ExecApprovalRequest(req) => Some(FilteredEvent::ApprovalRequest {
                command: format!("{command:?}", command = req.command),
                call_id: req.call_id.clone(),
            }),
            EventMsg::ItemStarted(item) => Some(FilteredEvent::ItemStarted(format!(
                "{item:?}",
                item = item.item
            ))),
            EventMsg::ItemCompleted(item) => Some(FilteredEvent::ItemCompleted(format!(
                "{item:?}",
                item = item.item
            ))),
            // Ignore most other event types for simple filtering
            _ => None,
        }
    }
}

/// Simplified representation of agent events.
///
/// This enum provides a more approachable interface to Codex events
/// for common use cases, hiding internal complexity.
#[derive(Debug, Clone)]
pub enum FilteredEvent {
    /// Agent sent a message fragment (delta).
    MessageDelta(String),
    /// Turn completed successfully.
    TurnComplete {
        /// The last agent message, if any.
        last_message: Option<String>,
    },
    /// Turn was aborted.
    TurnAborted {
        /// Reason for abortion.
        reason: String,
    },
    /// An error occurred.
    Error(String),
    /// Agent requests approval to run a command.
    ApprovalRequest {
        /// The command to approve.
        command: String,
        /// Call ID to use when responding.
        call_id: String,
    },
    /// A turn item started.
    ItemStarted(String),
    /// A turn item completed.
    ItemCompleted(String),
}

/// Builder for constructing Codex operations easily.
///
/// This provides simple helper methods for common operations without
/// needing to understand all the details of the `Op` enum.
pub struct OpBuilder;

impl OpBuilder {
    /// Create a user message operation.
    ///
    /// This is the simplest way to send a message to the agent.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use codex_agent_loop::op_builder::OpBuilder;
    ///
    /// let op = OpBuilder::user_message("Hello, agent!");
    /// ```
    pub fn user_message(content: impl Into<String>) -> Op {
        Op::UserInput {
            items: vec![UserInput::Text {
                text: content.into(),
            }],
            final_output_json_schema: None,
        }
    }

    /// Create an interrupt operation to stop the current task.
    pub fn interrupt() -> Op {
        Op::Interrupt
    }

    /// Create an operation to approve a command execution.
    pub fn approve_exec(request_id: impl Into<String>) -> Op {
        use codex_protocol::protocol::ReviewDecision;
        Op::ExecApproval {
            id: request_id.into(),
            decision: ReviewDecision::Approved,
        }
    }

    /// Create an operation to deny a command execution.
    pub fn deny_exec(request_id: impl Into<String>) -> Op {
        use codex_protocol::protocol::ReviewDecision;
        Op::ExecApproval {
            id: request_id.into(),
            decision: ReviewDecision::Denied,
        }
    }
}
