pub mod installer;
pub mod manager;
pub mod manifest;
pub mod rpc;
pub mod store;

#[allow(unused_imports)]
pub use installer::{get_target_triple, install_plugin, safe_extract, uninstall_plugin};
#[allow(unused_imports)]
pub use manager::PluginManager;
#[allow(unused_imports)]
pub use manifest::{
    PluginEntryType, PluginManifest, PluginRuntimeInfo, PluginState, PluginUiConfig,
};
#[allow(unused_imports)]
pub use rpc::{RpcError, RpcRequest, RpcResponse};
#[allow(unused_imports)]
pub use store::{PluginDownloadAsset, PluginStore, PluginStoreCatalogItem, PluginStoreItemView};
