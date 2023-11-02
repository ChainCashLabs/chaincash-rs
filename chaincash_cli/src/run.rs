use anyhow::Result;
use chaincash_store::{ChainCashStore, Update};
use tracing::info;

#[derive(Debug, clap::Args)]
pub(crate) struct Args {
    /// The address to listen on
    #[clap(short, long, default_value = "127.0.0.1:8080")]
    listen: String,
}

pub(crate) async fn execute(args: &Args) -> Result<()> {
    // listenfd is used to enable auto-reloading in development
    // otherwise fallback to standard tcp listener
    let listener = listenfd::ListenFd::from_env()
        .take_tcp_listener(0)?
        .unwrap_or_else(|| std::net::TcpListener::bind(&args.listen).unwrap());

    info!("listening on {:?}", listener.local_addr()?);

    let db_path = std::env::current_dir()?.join("chaincash.sqlite");

    std::fs::create_dir_all(&db_path).unwrap();
    info!("using database path: {}", db_path.display());

    let store = ChainCashStore::open(db_path.to_string_lossy())?;

    if store.has_updates()? {
        info!("apply updates to chaincash store");
        store.update()?;
    }

    Ok(chaincash_server::serve_blocking(listener, store).await?)
}
