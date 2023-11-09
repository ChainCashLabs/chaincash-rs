use reqwest::Url;

pub use ergo_node_interface::NodeInterface;

#[derive(serde::Deserialize, Debug)]
pub struct Config {
    url: String,
    api_key: String,
}

pub fn node_from_config(cfg: &Config) -> NodeInterface {
    let url = Url::parse(&cfg.url).unwrap();
    NodeInterface::from_url(&cfg.api_key, url)
}
