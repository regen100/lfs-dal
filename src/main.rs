mod config;

use anyhow::Result;
use clap::Parser;
use lfs_dal::Agent;
use log::error;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Set log file name
    #[arg(long, value_name = "PATH")]
    log_output: Option<std::path::PathBuf>,

    /// Set log level
    #[arg(long, value_name = "LEVEL", default_value = "info")]
    log_level: log::Level,

    /// List enabled schemes
    #[arg(long)]
    list: bool,

    /// Exit immediately (for debugging OpenDAL settings)
    #[arg(long)]
    exit: bool,
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    if let Some(log_output) = cli.log_output {
        simplelog::WriteLogger::init(
            cli.log_level.to_level_filter(),
            simplelog::Config::default(),
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_output)?,
        )?;
    }
    if cli.list {
        let mut schemes: Vec<_> = opendal::Scheme::enabled()
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        schemes.sort();
        println!("{}", schemes.join("\n"));
        return Ok(());
    }

    let remote_op = config::remote_operator()?;

    if cli.exit {
        return Err(anyhow::anyhow!("no error"));
    }

    // setup agent
    let (tx, mut rx) = tokio::sync::mpsc::channel(32);
    let mut agent = Agent::new(remote_op, tx);
    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            let mut stdout = io::stdout();
            stdout
                .write_all(format!("{}\n", msg).as_bytes())
                .await
                .unwrap();
            stdout.flush().await.unwrap();
        }
    });

    let mut lines = io::BufReader::new(io::stdin()).lines();
    while let Some(line) = lines.next_line().await? {
        agent.process(&line).await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{}", e);
        error!("{}", e);
        std::process::exit(1);
    }
}
