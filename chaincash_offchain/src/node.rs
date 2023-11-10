pub use ergo_node_interface::NodeInterface;

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    url: String,
    api_key: String,
}

pub fn node_from_config(cfg: &Config) -> NodeInterface {
    NodeInterface::from_url_str(&cfg.api_key, &cfg.url).unwrap()
}
