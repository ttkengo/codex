//! Optional extension traits for custom LLM providers and tools.

use anyhow::Result;
use async_trait::async_trait;
use futures::Stream;

/// Trait for implementing custom LLM providers.
///
/// This allows using alternative LLM backends while still leveraging
/// the Codex infrastructure for tools, history, and sandboxing.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Stream a completion from the LLM.
    async fn stream_completion(
        &self,
        prompt: &str,
    ) -> Result<Box<dyn Stream<Item = Result<String>> + Send + Unpin>>;

    /// Get the model name.
    fn model_name(&self) -> &str;
}

/// Trait for custom tool extensions.
///
/// This allows registering custom tools that can be called by the agent.
#[async_trait]
pub trait ToolExtension: Send + Sync {
    /// Get the tool name.
    fn name(&self) -> &str;

    /// Get the tool description for the LLM.
    fn description(&self) -> &str;

    /// Get the JSON schema for the tool parameters.
    fn parameter_schema(&self) -> serde_json::Value;

    /// Execute the tool with the given arguments.
    async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value>;
}
