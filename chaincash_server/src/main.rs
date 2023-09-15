//! ChainCash server CLI.
use clap::Parser;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// ChainCash payment server
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The address to listen on
    #[clap(short, long, default_value = "127.0.0.1:8080")]
    listen: String,

    /// The level of logging to use for the server
    #[clap(long, default_value = tracing::Level::INFO.as_str())]
    log_level: tracing::Level,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                format!("chaincash_server={},axum::rejection=trace", args.log_level).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("started with {:?}", args);

    // listenfd is used to enable auto-reloading in development
    // otherwise fallback to standard tcp listener
    let listener = listenfd::ListenFd::from_env()
        .take_tcp_listener(0)
        .unwrap()
        .unwrap_or_else(|| std::net::TcpListener::bind(args.listen).unwrap());

    info!("listening on {:?}", listener.local_addr().unwrap());

    chaincash_server::serve_blocking(listener).await.unwrap();
}
