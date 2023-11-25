//! ChainCash server CLI.
use anyhow::Result;
use chaincash_app::{ChainCashApp, ChainCashConfig};
use clap::{Parser, Subcommand};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Subcommand)]
enum Command {
    /// Runs the chaincash server
    Run,
}

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// The level of logging to use for the server
    #[clap(long, global = true, default_value = tracing::Level::INFO.as_str())]
    log_level: tracing::Level,
}

impl Cli {
    pub async fn execute(&self) -> Result<()> {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                    // axum logs rejections from built-in extractors with the `axum::rejection`
                    // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                    format!(
                        "chaincash_cli={},chaincash_server={},axum::rejection=trace,reqwest={}",
                        self.log_level, self.log_level, self.log_level
                    )
                    .into()
                }),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();

        info!("started with {:?}", self);

        match &self.command {
            Command::Run => Ok(ChainCashApp::new(ChainCashConfig::new()?).run().await?),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Setup human panic
    human_panic::setup_panic!();

    Cli::parse().execute().await
}
