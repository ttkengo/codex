//! Consolidated demo for the agent loop with multiple modes.
//!
//! Run with:
//!   cargo run --example full_demo -p codex-agent-loop -- <mode> [options]
//!
//! Modes:
//!   full        Full demo with workspace + event loop
//!   hello       Minimal hello-world question
//!   auth        ChatGPT OAuth walkthrough

use anyhow::Context;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use codex_agent_loop::AgentBuilder;
use codex_agent_loop::SimpleAgent;
use codex_protocol::protocol::ElicitationAction;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::Op;
use codex_protocol::protocol::ReviewDecision;
use codex_protocol::user_input::UserInput;
use std::path::Path;
use std::path::PathBuf;

const DEFAULT_HELLO_PROMPT: &str = "What is 2 + 2? Just give me the number.";

#[derive(Parser)]
#[command(name = "full_demo")]
#[command(about = "Codex Agent Loop - Consolidated Demo", long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Full demo with workspace + event loop
    Full,
    /// Minimal hello-world question
    Hello {
        /// Prompt to send to the agent
        #[arg(long, default_value = DEFAULT_HELLO_PROMPT)]
        prompt: String,
    },
    /// ChatGPT OAuth walkthrough
    Auth,
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let args = Args::parse();
    let command = args.command.unwrap_or(Command::Full);

    match command {
        Command::Full => full_demo().await,
        Command::Hello { prompt } => hello_demo(&prompt).await,
        Command::Auth => auth_demo().await,
    }
}

async fn full_demo() -> Result<()> {
    println!("Codex Agent Loop - Full Demo");
    println!("============================\n");

    let temp_dir = tempfile::tempdir()?;
    let workspace = temp_dir.path();
    println!("Workspace: {path}\n", path = workspace.display());

    let agent = build_agent(Some(workspace), "gpt-5.1-codex-max").await?;
    println!("Agent created successfully!\n");

    run_demo_1(&agent).await?;
    println!("\n{}\n", "=".repeat(60));

    run_demo_2(&agent).await?;
    println!("\n{}\n", "=".repeat(60));

    run_demo_3(&agent).await?;

    println!("\nDemo complete.");
    Ok(())
}

async fn hello_demo(prompt: &str) -> Result<()> {
    println!("Creating agent with ChatGPT OAuth...");
    println!("(You may be prompted to log in if not already authenticated)\n");

    let agent = build_agent(None, "gpt-5.1-codex-max").await?;

    println!("Agent created successfully!\n");

    println!("Question: {prompt}");
    println!("Thinking...\n");

    let response = agent.send_message(prompt).await?;
    println!("Answer: {response}");
    Ok(())
}

async fn auth_demo() -> Result<()> {
    println!("Using ChatGPT Login (No API Key Required!)\n");

    let auth_file = codex_home()?.join("auth.json");
    if !auth_file.exists() {
        println!("Not logged in yet.");
        println!("To use ChatGPT login, run:");
        println!("  codex login");
        println!("This will open a browser and store credentials in ~/.codex/auth.json\n");
        return Ok(());
    }

    println!("Found existing authentication!\n");

    let agent = build_agent(Some(Path::new(".")), "gpt-5.1-codex-max")
        .await
        .context("Failed to create agent")?;

    println!("Agent created using ChatGPT authentication!\n");
    println!("Agent Details:");
    println!("  Thread ID: {}", agent.thread_id());
    println!("  Authentication: ChatGPT OAuth (from codex login)");
    println!("  Model: gpt-5.1-codex-max");
    println!("  Workspace: .\n");

    println!("Sending test message...");
    let response = agent
        .send_message("What is 2+2? Just give me the number.")
        .await
        .context("Failed to send message")?;

    println!("Response: {response}\n");
    Ok(())
}

async fn run_demo_1(agent: &SimpleAgent) -> Result<()> {
    println!("Demo 1: File Operations");
    println!("---------------------------");

    let task = "Create a file called demo.txt with the content 'Hello from Codex Agent!' \
                and then read it back to confirm.";

    println!("Task: {task}\n");
    println!("Working...\n");

    let response = agent.send_message(task).await?;

    println!("Response:");
    println!("{response}\n");

    Ok(())
}

async fn run_demo_2(agent: &SimpleAgent) -> Result<()> {
    println!("Demo 2: Multi-Step Task");
    println!("--------------------------");

    let task = "Create three files: data1.txt, data2.txt, and data3.txt. \
                Put the numbers 1, 2, and 3 in them respectively. \
                Then list all .txt files and confirm they were created.";

    println!("Task: {task}\n");
    println!("Working...\n");

    let response = agent.send_message(task).await?;

    println!("Response:");
    println!("{response}\n");

    Ok(())
}

async fn run_demo_3(agent: &SimpleAgent) -> Result<()> {
    use std::io::Write;

    println!("Demo 3: Event-Driven Interaction");
    println!("------------------------------------");
    println!("This shows the full event loop with real-time updates\n");

    let task = "Calculate the sum of numbers from 1 to 10 and create a file called result.txt \
                with the answer.";

    println!("Task: {task}\n");

    agent
        .submit(Op::UserInput {
            items: vec![UserInput::Text {
                text: task.to_string(),
            }],
            final_output_json_schema: None,
        })
        .await?;

    let mut message_buffer = String::new();
    let mut event_count = 0;

    println!("Event Stream:");
    println!("----------------");

    loop {
        let event = agent.next_event().await?;
        event_count += 1;

        match event.msg {
            EventMsg::AgentMessageContentDelta(delta) => {
                print!("{}", delta.delta);
                std::io::stdout().flush()?;
                message_buffer.push_str(&delta.delta);
            }
            EventMsg::AgentMessageDelta(delta) => {
                print!("{}", delta.delta);
                std::io::stdout().flush()?;
                message_buffer.push_str(&delta.delta);
            }
            EventMsg::TurnComplete(_) => {
                println!("\n\nTurn completed!");
                println!("Total events processed: {event_count}");
                break;
            }
            EventMsg::TurnAborted(abort) => {
                println!("\nTurn aborted: {reason:?}", reason = abort.reason);
                break;
            }
            EventMsg::Error(err) => {
                println!("\nError: {message}", message = err.message);
                break;
            }
            EventMsg::ItemStarted(item) => {
                println!("\nItem started: {item:?}", item = item.item);
            }
            EventMsg::ItemCompleted(item) => {
                println!("Item completed: {item:?}", item = item.item);
            }
            EventMsg::ExecApprovalRequest(req) => {
                println!(
                    "\nApproval requested for: {command:?}",
                    command = req.command
                );
                println!("Auto-approving for demo purposes");

                agent
                    .submit(Op::ExecApproval {
                        id: req.call_id,
                        decision: ReviewDecision::Approved,
                    })
                    .await?;
            }
            EventMsg::ApplyPatchApprovalRequest(req) => {
                println!(
                    "\nPatch approval requested for {count} file(s)",
                    count = req.changes.len()
                );
                println!("Auto-approving for demo purposes");

                agent
                    .submit(Op::PatchApproval {
                        id: req.call_id,
                        decision: ReviewDecision::Approved,
                    })
                    .await?;
            }
            EventMsg::ElicitationRequest(req) => {
                println!(
                    "\nElicitation request from {server}",
                    server = req.server_name
                );
                println!("Auto-accepting for demo purposes");

                agent
                    .submit(Op::ResolveElicitation {
                        server_name: req.server_name,
                        request_id: req.id,
                        decision: ElicitationAction::Accept,
                    })
                    .await?;
            }
            _ => {}
        }
    }

    println!("\nFinal message:");
    println!("{message_buffer}");

    Ok(())
}

async fn build_agent(workspace: Option<&Path>, model: &str) -> Result<SimpleAgent> {
    let mut builder = AgentBuilder::new().with_model(model);
    if let Some(path) = workspace {
        builder = builder.with_workspace(path);
    }
    builder.build().await
}

fn codex_home() -> Result<PathBuf> {
    Ok(dirs::home_dir()
        .context("Could not find home directory")?
        .join(".codex"))
}
