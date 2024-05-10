use plugin_host::{PluginError, PluginFactory, PluginId, PluginProxyObj, wasi::WasiPluginFactory};
use plugin_rpc::{*, source::*};
use sources::{ByteSource, ReloadInfo, SourceFilter, Error as SourceError};
use std::{fs, path::{PathBuf, Path}};
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
    stats: ByteSourceStats,
    content: Vec<u8>,
}

impl ByteSourceProxy {
    pub fn new(mut proxy: PluginProxyObj, input_path: &Path, total_capacity: usize, buffer_min: usize) -> Self {
        println!("\x1b[93mhost : new byte-source proxy<{}>\x1b[0m", proxy.id());

        let request: PluginRequest<ByteSourceRequest> = PluginRequest::Plugin(
            ByteSourceRequest::Setup(ByteSourceSettings {
                input_path: input_path.display().to_string(),
                total_capacity,
                buffer_min
            }));
        let request_bytes = rkyv::to_bytes::<_, 256>(&request).unwrap();

        match proxy.call(&request_bytes) {
            Ok(response_bytes) => {
                let response: PluginResponse<ByteSourceResponse> = rkyv::from_bytes(&response_bytes).unwrap();
                if let PluginResponse::Plugin(ByteSourceResponse::SetupDone) = response {
                    // nothing
                } else {
                    panic!("source-plugin: unexpected response: #{}", response);
                }
            }
            _ => {
                panic!("source-plugin: request failed");
            }
        }

        Self {
            proxy, 
            stats: ByteSourceStats::default(),
            content: vec![]
        }
    }
}

#[async_trait]
impl ByteSource for ByteSourceProxy {
    fn consume(&mut self, offset: usize) {
        self.stats.calls_consume += 1;

        let request: PluginRequest<ByteSourceRequest> = PluginRequest::Plugin(
            ByteSourceRequest::Consume(offset));
        let request_bytes = rkyv::to_bytes::<_, 256>(&request).unwrap();

        match self.proxy.call(&request_bytes) {
            Ok(response_bytes) => {
                let response: PluginResponse<ByteSourceResponse> = rkyv::from_bytes(&response_bytes).unwrap();
                if let PluginResponse::Plugin(ByteSourceResponse::ConsumeDone) = response {
                    // nothing
                } else {
                    panic!("source-plugin: unexpected response: #{}", response);
                }
            }
            _ => {
                panic!("source-plugin: request failed");
            }
        }
    }

    async fn reload(&mut self, _filter: Option<&SourceFilter>) -> Result<Option<ReloadInfo>, SourceError> {
        self.stats.calls_reload += 1;

        let request: PluginRequest<ByteSourceRequest> = PluginRequest::Plugin(
            ByteSourceRequest::Reload);
        let request_bytes = rkyv::to_bytes::<_, 256>(&request).unwrap();

        match self.proxy.call(&request_bytes) {
            Ok(response_bytes) => {
                let response: PluginResponse<ByteSourceResponse> = rkyv::from_bytes(&response_bytes).unwrap();
                if let PluginResponse::Plugin(ByteSourceResponse::ReloadResult(result)) = response {
                    match result {
                        SourceReloadResult::ReloadOk(result) => {
                            self.stats.reload_ok += 1;
                            self.content = result.bytes;
                            return Ok(Some(ReloadInfo::new(
                                result.newly_loaded_bytes,
                                result.available_bytes,
                                result.skipped_bytes,
                                None,
                            )));
                        },
                        SourceReloadResult::ReloadEof => {
                            self.stats.reload_eof += 1;
                            return Ok(None);
                        }
                        SourceReloadResult::ReloadError(error) => {
                            println!("\x1b[93mhost : return error from proxy<{}> : {}\x1b[0m", self.proxy.id(), error);
                            self.stats.reload_error += 1;
                            return Err(SourceError::Unrecoverable(error));
                        }
                    }
                } else {
                    panic!("source-plugin: unexpected response: #{}", response);
                }
            }
            _ => {
                panic!("source-plugin: request failed");
            }
        }
    }

    fn current_slice(&self) -> &[u8] {
        &self.content
    }

    fn len(&self) -> usize {
        self.content.len()
    }
}

impl std::ops::Drop for ByteSourceProxy {
    fn drop(&mut self) {
        println!("\x1b[93mhost : proxy<{}> stats : {}\x1b[0m", self.proxy.id(), self.stats);
    }
}

#[derive(Default)]
struct ByteSourceStats {
    pub calls_consume: usize,
    pub calls_reload: usize,
    pub reload_ok: usize,
    pub reload_eof: usize,
    pub reload_error: usize,
}

impl std::fmt::Display for ByteSourceStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "c-cns {}, c-rld {}, r-ok {}, r-eof {}, r-err {}", 
            self.calls_consume,
            self.calls_reload,
            self.reload_ok,
            self.reload_eof,
            self.reload_error
        )
    }
}
