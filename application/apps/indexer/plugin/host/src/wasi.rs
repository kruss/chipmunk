use crate::{PluginEnv, PluginError, PluginFactory, PluginId, PluginProxy};
use log::{debug, info};
use std::{borrow::Borrow, path::Path};
use wasmer::{AsStoreRef, Function, FunctionEnv, FunctionEnvMut, Instance, Module, Store, WasmPtr};
use wasmer_wasix::{default_fs_backing, WasiEnv};

pub struct WasiPluginFactory {
    binary: Vec<u8>,
}

impl WasiPluginFactory {
    pub fn new(binary: Vec<u8>) -> Self {
        WasiPluginFactory { binary }
    }
}

impl PluginFactory for WasiPluginFactory {
    fn create(&self, id: PluginId) -> Result<PluginProxy, PluginError> {
        info!("create wasi proxy<{}>", id);

        let mut store = Store::default();
        let module = Module::from_binary(&store, &self.binary).expect("compile");

        let mut wasi_env = WasiEnv::builder(format!("wasi-proxy<{}>", id))
            .fs(default_fs_backing())
            .preopen_dir(Path::new("/"))
            .expect("preopen_dir")
            .map_dir("/", ".")
            .expect("map_dir")
            .finalize(&mut store)
            .expect("finalize_env");

        let mut imports = wasi_env
            .import_object(&mut store, &module)
            .expect("imports");

        let plugin_env = FunctionEnv::new(&mut store, PluginEnv { id, memory: None });

        let host_print = Function::new_typed_with_env(
            &mut store,
            &plugin_env,
            |env: FunctionEnvMut<PluginEnv>, ptr: WasmPtr<u8>, len: u32| {
                let store = env.as_store_ref();
                let memory = env.data().memory.as_ref().unwrap();
                let memory_view = memory.view(store.borrow());
                let string = ptr.read_utf8_string(&memory_view, len).unwrap();
                //debug!("proxy<{}> : {}", env.data().id, string);
                println!("\x1b[93mproxy<{}> : {}\x1b[0m", env.data().id, string);
            },
        );

        imports.define("host", "host_print", host_print);

        let instance = Instance::new(&mut store, &module, &imports).expect("instance");

        let memory = instance.exports.get_memory("memory").expect("memory");
        //memory.grow(&mut store, 1024).expect("grow"); // TODO via config or dynamically
        let memory_view = memory.view(&store);
        debug!("wasm memory: {:?} bytes", memory_view.data_size());

        wasi_env
            .initialize(&mut store, instance.clone())
            .expect("initialize_env");

        let plugin_env = plugin_env.as_mut(&mut store);
        plugin_env.memory = Some(
            instance
                .exports
                .get_memory("memory")
                .expect("memory")
                .clone(),
        );

        let start = instance
            .exports
            .get_function("_initialize")
            .expect("exports");
        start.call(&mut store, &[]).expect("start");

        //wasi_env.on_exit(&mut store, None);

        Ok(PluginProxy::new(id, store, instance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_plugin() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/wasi/plugin.wasm");
        std::fs::read(path).unwrap()
    }

    #[test]
    fn test_wasi_proxy() {
        let _ = env_logger::try_init();

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let _guard = runtime.enter();

        let binary = load_plugin();
        let factory = WasiPluginFactory::new(binary);

        let id = 0;
        let mut proxy = factory.create(id).expect("proxy");
        assert_eq!(id, proxy.id);

        let request = [0, 1, 2].to_vec();
        let response = proxy.call(&request).expect("call");
        assert_eq!(request, response);
    }
}
