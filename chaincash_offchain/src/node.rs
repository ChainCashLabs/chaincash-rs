use reqwest::Url;

pub use ergo_node_interface::NodeInterface;

#[derive(serde::Deserialize, Debug)]
pub struct NodeConfig {
    url: String,
    api_key: String,
}
