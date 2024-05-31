use crate::{PluginEnv, PluginError, PluginFactory, PluginId, PluginProxy};
use log::{debug, info};
use std::borrow::Borrow;
use wasmer::{
    imports, AsStoreRef, Function, FunctionEnv, FunctionEnvMut, Instance, Module, Store, WasmPtr,
};

pub struct WasmPluginFactory {
    binary: Vec<u8>,
}

impl WasmPluginFactory {
    pub fn new(binary: Vec<u8>) -> Self {
        WasmPluginFactory { binary }
    }
}

impl PluginFactory for WasmPluginFactory {
    fn create(&self, id: PluginId) -> Result<PluginProxy, PluginError> {
        info!("create wasm proxy<{}>", id);

        let mut store = Store::default();
        let module = Module::from_binary(&store, &self.binary).expect("compile");

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

        let imports = imports! {
            "host" => {
                "host_print" => host_print,
            },
            // Note: Imports due dependencies:
            "__wbindgen_placeholder__" => {
                "__wbindgen_describe" => Function::new_typed(&mut store, |_: i32| { todo!() }),
            },
            "__wbindgen_placeholder__" => {
                "__wbindgen_throw" => Function::new_typed(&mut store, |_: i32, _: i32| { todo!() }),
            },
            "__wbindgen_externref_xform__" => {
                "__wbindgen_externref_table_grow" => Function::new_typed(&mut store, |_: i32| -> i32 { todo!() }),
                "__wbindgen_externref_table_set_null" => Function::new_typed(&mut store, |_: i32| { todo!() }),
            },
        };

        let instance = Instance::new(&mut store, &module, &imports).expect("instance");

        let memory = instance.exports.get_memory("memory").expect("memory");
        //memory.grow(&mut store, 1024).expect("grow"); // TODO via config or dynamically
        let memory_view = memory.view(&store);
        debug!("wasm memory: {:?} bytes", memory_view.data_size());

        let plugin_env = plugin_env.as_mut(&mut store);
        plugin_env.memory = Some(memory.clone());

        Ok(PluginProxy::new(id, store, instance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn load_plugin() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/wasm/plugin.wasm");
        std::fs::read(path).unwrap()
    }

    #[test]
    fn test_wasm_proxy() {
        let _ = env_logger::try_init();

        let binary = load_plugin();
        let factory = WasmPluginFactory::new(binary);

        let id = 0;
        let mut proxy = factory.create(id).expect("proxy");
        assert_eq!(id, proxy.id);

        let request = [0, 1, 2].to_vec();
        let response = proxy.call(&request).expect("call");
        assert_eq!(request, response);
    }
}
