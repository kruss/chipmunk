use crate::{PluginError, PluginFactory, PluginId, PluginProxy, PluginProxyObj};
use byteorder::{ByteOrder, LittleEndian};
use log::{debug, info, trace};
use std::{any::Any, borrow::Borrow, mem::size_of};
use wasmer::{
    imports, AsStoreRef, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, Module, Store,
    TypedFunction, WasmPtr, WasmSlice,
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
    fn create(&self, id: PluginId) -> Result<PluginProxyObj, PluginError> {
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

        Ok(Box::new(WasmProxy::new(id, store, instance)))
    }
}

#[derive(Clone, Debug)]
struct PluginEnv {
    id: PluginId,
    memory: Option<Memory>,
}

pub struct WasmProxy {
    id: PluginId,
    store: Store,
    instance: Instance,
    message: TypedFunction<(WasmPtr<u8>, WasmPtr<u8>, u32), ()>,
}

impl WasmProxy {
    pub fn new(id: PluginId, store: Store, instance: Instance) -> Self {
        let message: TypedFunction<(WasmPtr<u8>, WasmPtr<u8>, u32), ()> = instance
            .exports
            .get_typed_function(&store, "message")
            .expect("function");

        WasmProxy {
            id,
            store,
            instance,
            message,
        }
    }
}

impl PluginProxy for WasmProxy {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn id(&self) -> PluginId {
        self.id
    }

    fn call(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginError> {
        debug!("proxy<{}> : send request: {} bytes", self.id, request.len());
        trace!("{:?}", request);

        let output_offset: u32 = 0;
        let output_len = 2 * size_of::<u32>() as u32;
        let input_offset: u32 = output_len; // 4Byte aligned!

        let memory = self.instance.exports.get_memory("memory").expect("memory");
        {
            let memory_view = memory.view(&self.store);
            //println!("wasm memory:     {:?} bytes", memory_view.data_size());
            //println!("wasm request:    {:?} bytes", request.len());
            memory_view
                .write(input_offset.into(), request)
                .expect("write");
        }

        let output_ptr = WasmPtr::new(output_offset);
        let input_ptr = WasmPtr::new(input_offset);

        //println!("wasm output_ptr: {:?}", output_ptr);
        //println!("wasm input_ptr:  {:?}", input_ptr);
        //println!("wasm input_len:  {:?}", request.len());

        self.message
            .call(&mut self.store, output_ptr, input_ptr, request.len() as u32)
            .expect("call");

        let memory = self.instance.exports.get_memory("memory").expect("memory");
        let memory_view = memory.view(&self.store);

        let slice: WasmSlice<'_, u8> = output_ptr.slice(&memory_view, output_len).unwrap();

        let bytes = slice.read_to_bytes().unwrap();
        let buffer: &[u8] = bytes.as_ref();

        let addr: u32 = LittleEndian::read_u32(&buffer[..size_of::<u32>()]);
        let len: u32 = LittleEndian::read_u32(&buffer[size_of::<u32>()..]);
        let ptr: WasmPtr<u8> = WasmPtr::new(addr);
        let data: WasmSlice<'_, u8> = ptr.slice(&memory_view, len).unwrap();

        let response = data.read_to_vec().unwrap();
        debug!(
            "proxy<{}> : received response: {} bytes",
            self.id,
            response.len()
        );
        trace!("{:?}", response);

        Ok(response)
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
        assert_eq!(id, proxy.id());

        proxy.init().expect("init");
    }
}
