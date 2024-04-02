use plugin_host::{PluginError, PluginFactory, PluginId, PluginProxyObj, wasi::WasiPluginFactory};
use plugin_rpc::{*, source::*};
use sources::{ByteSource, ReloadInfo, SourceFilter, Error as SourceError};
use std::{fs, path::PathBuf};
use std::io::Read;
use std::io::Seek;
use async_trait::async_trait;

// Plugin factory that will create a WASI byte-source plugin.
// TODO: Load WASI binary via configuration.
pub struct SourcePluginFactory {
    factory: WasiPluginFactory
}

impl SourcePluginFactory {
    pub fn new() -> Self {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../plugin/source/target/wasm32-wasi/release/plugin.wasm");
        let binary = fs::read(path).unwrap();
        
        SourcePluginFactory { 
            factory: WasiPluginFactory::new(binary)
        }
    }
}

impl PluginFactory for SourcePluginFactory {
    fn create(&self, id: PluginId) -> Result<PluginProxyObj, PluginError> {
        self.factory.create(id)
    }
}

pub struct ByteSourceProxy {
    proxy: PluginProxyObj,
}

impl ByteSourceProxy {
    pub fn new(mut proxy: PluginProxyObj) -> Self {
        println!("\x1b[93mhost : new proxy<{}>\x1b[0m", proxy.id());
        Self {
            proxy,
        }
    }
}

#[async_trait]
impl ByteSource for ByteSourceProxy {
    fn consume(&mut self, offset: usize) {
        todo!();
    }

    fn current_slice(&self) -> &[u8] {
        todo!();
    }

    fn len(&self) -> usize {
        todo!();
    }

    async fn reload(&mut self, filter: Option<&SourceFilter>) -> Result<Option<ReloadInfo>, SourceError> {
        todo!();
    }
}
