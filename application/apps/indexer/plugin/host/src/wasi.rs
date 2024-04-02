use crate::{PluginError, PluginFactory, PluginId, PluginProxy, PluginProxyObj};
use byteorder::{ByteOrder, LittleEndian};
use log::{debug, info, trace};
use std::{any::Any, borrow::Borrow, mem::size_of, path::Path};
use wasmer::{
    AsStoreRef, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, Module, Store,
    TypedFunction, WasmPtr, WasmSlice,
};
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
    fn create(&self, id: PluginId) -> Result<PluginProxyObj, PluginError> {
        info!("create wasi proxy<{}>", id);

        let mut store = Store::default();
        let module = Module::from_binary(&store, &self.binary).expect("compile");

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

        let _guard = runtime.enter();

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
        let plugin_print = Function::new_typed_with_env(
            &mut store,
            &plugin_env,
            |env: FunctionEnvMut<PluginEnv>, ptr: WasmPtr<u8>, len: u32| {
                let store = env.as_store_ref();
                let memory = env.data().memory.as_ref().unwrap();
                let memory_view = memory.view(store.borrow());
                let string = ptr.read_utf8_string(&memory_view, len).unwrap();
                debug!("proxy<{}> : {}", env.data().id, string);
            },
        );

        imports.define("host", "host_print", plugin_print);

        let instance = Instance::new(&mut store, &module, &imports).expect("instance");

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

        let start = instance.exports.get_function("_start").expect("exports");
        start.call(&mut store, &[]).expect("start");

        wasi_env.on_exit(&mut store, None);

        Ok(Box::new(WasiProxy::new(id, store, instance)))
    }
}

#[derive(Clone, Debug)]
struct PluginEnv {
    id: PluginId,
    memory: Option<Memory>,
}

#[allow(dead_code)]
pub struct WasiProxy {
    id: PluginId,
    store: Store,
    instance: Instance,
    message: TypedFunction<(WasmPtr<u8>, WasmPtr<u8>, u32), ()>,
}

impl WasiProxy {
    pub fn new(id: PluginId, store: Store, instance: Instance) -> Self {
        let message: TypedFunction<(WasmPtr<u8>, WasmPtr<u8>, u32), ()> = instance
            .exports
            .get_typed_function(&store, "message")
            .expect("function");

        WasiProxy {
            id,
            store,
            instance,
            message,
        }
    }
}

impl PluginProxy for WasiProxy {
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
            memory_view
                .write(input_offset.into(), request)
                .expect("write");
        }

        let output_ptr = WasmPtr::new(output_offset);
        let input_ptr = WasmPtr::new(input_offset);

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
        path.push("tests/wasi/plugin.wasm");
        std::fs::read(path).unwrap()
    }

    #[test]
    fn test_wasi_proxy() {
        let _ = env_logger::try_init();

        let binary = load_plugin();
        let factory = WasiPluginFactory::new(binary);

        let id = 0;
        let mut proxy = factory.create(id).expect("proxy");
        assert_eq!(id, proxy.id());

        proxy.init().expect("init");
    }
}
