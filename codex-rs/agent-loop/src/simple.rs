//! Simple agent wrapper around Codex.

use anyhow::Context;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;

use codex_core::AuthManager;
use codex_core::CodexThread;
use codex_core::ThreadManager;
use codex_core::config::ConfigBuilder;
use codex_core::config::ConfigOverrides;
use codex_protocol::ThreadId;
use codex_protocol::protocol::ElicitationAction;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::ReviewDecision;
use codex_protocol::protocol::SessionSource;
use codex_protocol::user_input::UserInput;
use tracing::debug;
use tracing::error;
use tracing::info;
use tracing::warn;

use crate::AgentBuilder;

/// High-level wrapper around `codex::Codex` with simplified configuration
/// and convenience methods.
///
/// This struct provides an easier-to-use interface for common agent operations
/// while still allowing access to the underlying Codex instance for advanced usage.
///
/// # Examples
///
/// ```rust,no_run
/// use codex_agent_loop::SimpleAgent;
///
/// # async fn example() -> anyhow::Result<()> {
/// let agent = SimpleAgent::builder()
///     .with_api_key("your-api-key")
///     .with_model("gpt-4")
///     .with_workspace("./my-project")
///     .build()
///     .await?;
///
/// let response = agent.send_message("Hello!").await?;
/// println!("{}", response);
/// # Ok(())
/// # }
/// ```
pub struct SimpleAgent {
    #[allow(dead_code)]
    thread_manager: ThreadManager,
    thread: Arc<CodexThread>,
    thread_id: ThreadId,
}

impl SimpleAgent {
    /// Create a new agent builder.
    pub fn builder() -> AgentBuilder {
        AgentBuilder::new()
    }

    /// Create a SimpleAgent from a configured builder.
    pub(crate) async fn from_builder(builder: AgentBuilder) -> Result<Self> {
        // Determine codex home directory
        let codex_home = dirs::home_dir()
            .context("Could not determine home directory")?
            .join(".codex");

        // Create auth manager
        use codex_core::auth::AuthCredentialsStoreMode;

        let auth_manager = if let Some(api_key) = builder.get_api_key() {
            if std::env::var("CODEX_API_KEY")
                .ok()
                .filter(|existing| existing != api_key)
                .is_some()
            {
                warn!("Overriding existing CODEX_API_KEY with builder-provided key");
            }
            // API key mode: Set via env var and create AuthManager
            // Safety: We control this environment variable and it's set before any multithreading
            unsafe {
                std::env::set_var("CODEX_API_KEY", api_key);
            }
            info!("Using API key authentication");
            Arc::new(AuthManager::new(
                codex_home.clone(),
                true, // enable_codex_api_key_env
                AuthCredentialsStoreMode::Auto,
            ))
        } else if builder.use_existing_auth() {
            // ChatGPT OAuth mode: Use existing credentials
            // This loads from ~/.codex/auth.json which contains either:
            // 1. OAuth tokens from `codex login` (ChatGPT account)
            // 2. API key if manually set
            // 3. Falls back to CODEX_API_KEY env var
            info!("Using existing authentication");
            Arc::new(AuthManager::new(
                codex_home.clone(),
                true, // enable_codex_api_key_env
                AuthCredentialsStoreMode::Auto,
            ))
        } else {
            // No auth provided - will fail if no existing auth found
            info!("No explicit auth provided; using existing authentication if available");
            Arc::new(AuthManager::new(
                codex_home.clone(),
                true,
                AuthCredentialsStoreMode::Auto,
            ))
        };

        // Create thread manager
        let thread_manager = ThreadManager::new(
            codex_home.clone(),
            auth_manager.clone(),
            SessionSource::Exec,
        );

        // Build config - use CLI overrides for builder settings
        let mut cli_overrides = Vec::new();
        let requested_workspace = builder.get_workspace().cloned();

        if let Some(model) = builder.get_model() {
            cli_overrides.push(("model".to_string(), toml::Value::String(model.to_string())));
        }

        let mut overrides = ConfigOverrides::default();
        if let Some(workspace) =
            resolve_workspace_hint(requested_workspace.as_ref(), builder.get_config_file()).await?
        {
            overrides.cwd = Some(workspace);
        }

        let config = ConfigBuilder::default()
            .codex_home(codex_home.clone())
            .cli_overrides(cli_overrides)
            .harness_overrides(overrides)
            .build()
            .await
            .context("Failed to load config")?;

        let mut config = config;
        if let Some(workspace) = requested_workspace {
            config.cwd = workspace
                .canonicalize()
                .context("Failed to canonicalize workspace path")?;
        }
        debug!(
            cwd = %config.cwd.display(),
            model = ?config.model,
            "Resolved agent configuration"
        );

        // Create a new thread
        let new_thread = thread_manager
            .start_thread(config)
            .await
            .context("Failed to create thread")?;

        Ok(Self {
            thread_manager,
            thread: new_thread.thread,
            thread_id: new_thread.thread_id,
        })
    }

    /// Access the underlying CodexThread for advanced usage.
    pub fn thread(&self) -> &Arc<CodexThread> {
        &self.thread
    }

    /// Get the thread ID.
    pub fn thread_id(&self) -> ThreadId {
        self.thread_id
    }

    /// Send a message to the agent and wait for a complete response.
    ///
    /// This is a convenience method that submits a user message and collects
    /// all agent responses until the turn is complete.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use codex_agent_loop::SimpleAgent;
    /// # async fn example(agent: &SimpleAgent) -> anyhow::Result<()> {
    /// let response = agent.send_message("Write a hello world program").await?;
    /// println!("Agent: {}", response);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn send_message(&self, message: impl AsRef<str>) -> Result<String> {
        let message_str = message.as_ref();

        // Create user input
        let input = UserInput::Text {
            text: message_str.to_string(),
        };

        // Submit as UserInput (simpler than UserTurn)
        let submission_id = self
            .thread
            .submit(Op::UserInput {
                items: vec![input],
                final_output_json_schema: None,
            })
            .await?;
        info!(submission_id = %submission_id, "Submitted user input");

        // Collect responses until turn completion
        let mut response_text = String::new();
        let mut last_message = None;

        loop {
            let event = self.thread.next_event().await?;
            if self
                .handle_event(event, Some(&mut response_text), &mut last_message)
                .await?
            {
                break;
            };
        }

        Ok(response_text)
    }

    /// Wait for the current task to complete and return when done.
    pub async fn wait_for_completion(&self) -> Result<()> {
        let mut last_message = None;
        loop {
            let event = self.thread.next_event().await?;
            if self.handle_event(event, None, &mut last_message).await? {
                return Ok(());
            }
        }
    }

    /// Get the conversation history by reading the rollout.
    ///
    /// This returns a simplified string representation of the conversation.
    pub async fn get_conversation(&self) -> Result<String> {
        let rollout_path = self.thread.rollout_path();
        let contents = tokio::fs::read_to_string(&rollout_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to read rollout file {path}",
                    path = rollout_path.display()
                )
            })?;
        Ok(contents)
    }

    /// Submit a raw operation to the underlying Codex thread.
    ///
    /// This provides direct access to the full Codex API for advanced usage.
    pub async fn submit(&self, op: Op) -> Result<String> {
        Ok(self.thread.submit(op).await?)
    }

    /// Get the next event from the agent.
    ///
    /// This provides direct access to the event stream for advanced usage.
    pub async fn next_event(&self) -> Result<codex_protocol::protocol::Event> {
        Ok(self.thread.next_event().await?)
    }

    async fn handle_event(
        &self,
        event: codex_protocol::protocol::Event,
        mut response_text: Option<&mut String>,
        last_message: &mut Option<String>,
    ) -> Result<bool> {
        let event_id = event.id;
        match event.msg {
            EventMsg::AgentMessageContentDelta(delta) => {
                if let Some(text) = response_text.as_deref_mut() {
                    text.push_str(&delta.delta);
                }
                debug!(
                    event_id = %event_id,
                    delta_len = delta.delta.len(),
                    "Agent message content delta"
                );
            }
            EventMsg::AgentMessageDelta(delta) => {
                if let Some(text) = response_text.as_deref_mut() {
                    text.push_str(&delta.delta);
                }
                debug!(
                    event_id = %event_id,
                    delta_len = delta.delta.len(),
                    "Agent message delta"
                );
            }
            EventMsg::AgentMessage(message) => {
                if let Some(text) = response_text.as_deref_mut()
                    && text.is_empty()
                {
                    text.push_str(&message.message);
                }
                *last_message = Some(message.message);
                debug!(event_id = %event_id, "Agent message received");
            }
            EventMsg::TurnComplete(complete) => {
                if let Some(text) = response_text
                    && text.is_empty()
                    && let Some(last) = complete.last_agent_message.or_else(|| last_message.clone())
                {
                    text.push_str(&last);
                }
                info!(event_id = %event_id, "Turn complete");
                return Ok(true);
            }
            EventMsg::TurnAborted(abort) => {
                error!(
                    event_id = %event_id,
                    reason = ?abort.reason,
                    "Turn aborted"
                );
                anyhow::bail!("Turn was aborted: {reason:?}", reason = abort.reason);
            }
            EventMsg::Error(err) => {
                error!(event_id = %event_id, error = %err.message, "Turn error");
                anyhow::bail!("Error: {message}", message = err.message);
            }
            EventMsg::Warning(warn_event) => {
                warn!(event_id = %event_id, warning = %warn_event.message, "Turn warning");
            }
            EventMsg::StreamError(err) => {
                warn!(event_id = %event_id, error = %err.message, "Stream error");
            }
            EventMsg::ExecApprovalRequest(req) => {
                info!(
                    event_id = %event_id,
                    call_id = %req.call_id,
                    command = ?req.command,
                    "Auto-approving exec request"
                );
                self.thread
                    .submit(Op::ExecApproval {
                        id: req.call_id,
                        decision: ReviewDecision::Approved,
                    })
                    .await?;
            }
            EventMsg::ApplyPatchApprovalRequest(req) => {
                info!(
                    event_id = %event_id,
                    call_id = %req.call_id,
                    change_count = req.changes.len(),
                    "Auto-approving patch request"
                );
                self.thread
                    .submit(Op::PatchApproval {
                        id: req.call_id,
                        decision: ReviewDecision::Approved,
                    })
                    .await?;
            }
            EventMsg::ElicitationRequest(req) => {
                info!(
                    event_id = %event_id,
                    server_name = %req.server_name,
                    request_id = ?req.id,
                    "Auto-accepting elicitation request"
                );
                self.thread
                    .submit(Op::ResolveElicitation {
                        server_name: req.server_name,
                        request_id: req.id,
                        decision: ElicitationAction::Accept,
                    })
                    .await?;
            }
            other => {
                debug!(event_id = %event_id, event = ?other, "Ignoring event");
            }
        }

        Ok(false)
    }
}

async fn resolve_workspace_hint(
    explicit_workspace: Option<&PathBuf>,
    config_file: Option<&PathBuf>,
) -> Result<Option<PathBuf>> {
    if let Some(workspace) = explicit_workspace {
        return Ok(Some(workspace.clone()));
    }

    let Some(config_file) = config_file else {
        return Ok(None);
    };

    let metadata = tokio::fs::metadata(config_file).await.with_context(|| {
        format!(
            "Failed to read config file {path}",
            path = config_file.display()
        )
    })?;
    if !metadata.is_file() {
        anyhow::bail!(
            "Config file {path} is not a file",
            path = config_file.display()
        );
    }

    let parent = config_file
        .parent()
        .context("Config file must have a parent directory")?;
    if parent.file_name().is_some_and(|name| name == ".codex") {
        let workspace = parent
            .parent()
            .context("Config file must live under a .codex directory")?;
        return Ok(Some(workspace.to_path_buf()));
    }

    Ok(Some(parent.to_path_buf()))
}
