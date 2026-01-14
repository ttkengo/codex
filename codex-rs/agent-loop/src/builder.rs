//! Agent builder for fluent API configuration.

use anyhow::Result;
use std::path::PathBuf;

use crate::SimpleAgent;

/// Builder for creating and configuring agents with sensible defaults.
///
/// Supports two authentication modes:
/// 1. **ChatGPT OAuth** (default) - Uses existing login from `codex login`
/// 2. **API Key** - Uses OpenAI API key
///
/// # Examples
///
/// ## Using ChatGPT Login (Recommended)
///
/// ```rust,no_run
/// use codex_agent_loop::AgentBuilder;
///
/// # async fn example() -> anyhow::Result<()> {
/// // Uses existing ChatGPT login from `codex login`
/// let agent = AgentBuilder::new()
///     .with_model("gpt-4")
///     .with_workspace("./my-project")
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Using API Key
///
/// ```rust,no_run
/// use codex_agent_loop::AgentBuilder;
///
/// # async fn example() -> anyhow::Result<()> {
/// let agent = AgentBuilder::new()
///     .with_api_key("your-api-key")  // Optional: only if you want API key mode
///     .with_model("gpt-4")
///     .with_workspace("./my-project")
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct AgentBuilder {
    api_key: Option<String>,
    model: Option<String>,
    workspace: Option<PathBuf>,
    config_file: Option<PathBuf>,
    use_existing_auth: bool,
}

impl AgentBuilder {
    /// Create a new agent builder with default settings.
    ///
    /// By default, uses existing ChatGPT OAuth credentials from `codex login`.
    pub fn new() -> Self {
        Self {
            api_key: None,
            model: None,
            workspace: None,
            config_file: None,
            use_existing_auth: true,
        }
    }

    /// Set the API key for authentication.
    ///
    /// If not set, the agent will use existing ChatGPT OAuth credentials
    /// from `codex login` or the CODEX_API_KEY environment variable.
    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self.use_existing_auth = false;
        self
    }

    /// Use existing authentication from Codex.
    ///
    /// This is the default behavior. The agent will use:
    /// 1. CODEX_API_KEY environment variable if set
    /// 2. Existing ChatGPT OAuth credentials from `codex login`
    /// 3. API key from ~/.codex/auth.json if present
    pub fn with_existing_auth(mut self) -> Self {
        self.use_existing_auth = true;
        self.api_key = None;
        self
    }

    /// Set the model to use (e.g., "gpt-4", "gpt-3.5-turbo").
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the workspace directory for the agent.
    pub fn with_workspace(mut self, path: impl Into<PathBuf>) -> Self {
        self.workspace = Some(path.into());
        self
    }

    /// Load configuration from a file (alternative to individual settings).
    pub fn with_config_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_file = Some(path.into());
        self
    }

    /// Build the agent with the configured settings.
    pub async fn build(self) -> Result<SimpleAgent> {
        SimpleAgent::from_builder(self).await
    }

    // Getters for SimpleAgent to access during construction
    pub(crate) fn get_api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    pub(crate) fn use_existing_auth(&self) -> bool {
        self.use_existing_auth
    }

    pub(crate) fn get_model(&self) -> Option<&str> {
        self.model.as_deref()
    }

    pub(crate) fn get_workspace(&self) -> Option<&PathBuf> {
        self.workspace.as_ref()
    }

    #[allow(dead_code)]
    pub(crate) fn get_config_file(&self) -> Option<&PathBuf> {
        self.config_file.as_ref()
    }
}

impl Default for AgentBuilder {
    fn default() -> Self {
        Self::new()
    }
}
