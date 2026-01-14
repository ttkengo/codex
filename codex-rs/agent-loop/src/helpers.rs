//! Helper functions for common agent workflows.

use crate::AgentBuilder;
use anyhow::Context;
use anyhow::Result;

/// Ask the agent a single question and get a response.
///
/// This is a convenience function that creates a one-shot agent interaction.
/// It handles all the setup and teardown automatically.
///
/// # Examples
///
/// ```rust,no_run
/// use codex_agent_loop::ask_once;
///
/// # async fn example() -> anyhow::Result<()> {
/// let response = ask_once(
///     "What is the capital of France?",
///     "gpt-4",
///     "your-api-key"
/// ).await?;
/// println!("{}", response);
/// # Ok(())
/// # }
/// ```
pub async fn ask_once(
    question: impl AsRef<str>,
    model: impl AsRef<str>,
    api_key: impl AsRef<str>,
) -> Result<String> {
    let agent = AgentBuilder::new()
        .with_api_key(api_key.as_ref())
        .with_model(model.as_ref())
        .build()
        .await
        .context("Failed to create agent")?;

    agent
        .send_message(question.as_ref())
        .await
        .context("Failed to get response")
}

/// Run an interactive session with the agent.
///
/// This function provides a simple REPL-style interface for conversing with the agent.
/// Messages are read from stdin and responses are printed to stdout.
///
/// # Examples
///
/// ```rust,no_run
/// use codex_agent_loop::interactive_session;
///
/// # async fn example() -> anyhow::Result<()> {
/// interactive_session("./my-project", None::<String>).await?;
/// # Ok(())
/// # }
/// ```
pub async fn interactive_session(
    workspace: impl AsRef<str>,
    config_file: Option<impl AsRef<str>>,
) -> Result<()> {
    use std::io::Write;
    use std::io::{self};

    // Build agent
    let mut builder = AgentBuilder::new().with_workspace(workspace.as_ref());

    if let Some(config) = config_file {
        builder = builder.with_config_file(config.as_ref());
    }

    let agent = builder.build().await.context("Failed to create agent")?;

    let mut stdout = io::stdout();
    let mut stderr = io::stderr();
    writeln!(stdout, "Codex Agent - Interactive Session")?;
    writeln!(
        stdout,
        "Workspace: {workspace}",
        workspace = workspace.as_ref()
    )?;
    writeln!(stdout, "Type your messages (Ctrl+C to exit)\n")?;

    loop {
        // Read input
        write!(stdout, "> ")?;
        stdout.flush()?;

        let mut input = String::new();
        if io::stdin().read_line(&mut input)? == 0 {
            // EOF
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        // Handle special commands
        if input == "exit" || input == "quit" {
            break;
        }

        // Send message and get response
        match agent.send_message(input).await {
            Ok(response) => {
                writeln!(stdout, "\nAgent: {response}\n")?;
            }
            Err(e) => {
                writeln!(stderr, "Error: {e:#}")?;
            }
        }
    }

    writeln!(stdout, "Session ended.")?;
    Ok(())
}

/// Batch process multiple questions with the same agent.
///
/// This is useful when you have multiple related questions and want to
/// maintain context between them.
///
/// # Examples
///
/// ```rust,no_run
/// use codex_agent_loop::batch_ask;
///
/// # async fn example() -> anyhow::Result<()> {
/// let questions = vec![
///     "What files are in this directory?",
///     "Create a README.md file",
///     "What did you just create?",
/// ];
///
/// let responses = batch_ask(&questions, "gpt-4", "api-key").await?;
/// for (q, r) in questions.iter().zip(responses.iter()) {
///     println!("Q: {}", q);
///     println!("A: {}\n", r);
/// }
/// # Ok(())
/// # }
/// ```
pub async fn batch_ask(
    questions: &[impl AsRef<str>],
    model: impl AsRef<str>,
    api_key: impl AsRef<str>,
) -> Result<Vec<String>> {
    let agent = AgentBuilder::new()
        .with_api_key(api_key.as_ref())
        .with_model(model.as_ref())
        .build()
        .await
        .context("Failed to create agent")?;

    let mut responses = Vec::new();

    for question in questions {
        let response = agent
            .send_message(question.as_ref())
            .await
            .context("Failed to get response")?;
        responses.push(response);
    }

    Ok(responses)
}
