pub mod wasi;
pub mod wasm;

use byteorder::{ByteOrder, LittleEndian};
use log::{debug, trace};
use std::{fmt::Debug, mem::size_of};
use strum_macros::Display;
use wasmer::{Instance, Memory, Store, TypedFunction, WasmPtr, WasmSlice};

pub type PluginId = usize;

#[derive(Debug, Display, PartialEq)]
pub enum PluginError {
    Invalid,
    Unsupported,
}

pub trait PluginFactory {
    fn create(&self, id: PluginId) -> Result<PluginProxy, PluginError>;
}

#[derive(Clone, Debug)]
struct PluginEnv {
    id: PluginId,
    memory: Option<Memory>,
}

pub struct PluginProxy {
    id: PluginId,
    store: Store,
    instance: Instance,
    message: TypedFunction<(WasmPtr<u8>, WasmPtr<u8>, u32), ()>,
}

impl PluginProxy {
    pub fn new(id: PluginId, store: Store, instance: Instance) -> Self {
        let message: TypedFunction<(WasmPtr<u8>, WasmPtr<u8>, u32), ()> = instance
            .exports
            .get_typed_function(&store, "message")
            .expect("function");

        PluginProxy {
            id,
            store,
            instance,
            message,
        }
    }

    pub fn id(&self) -> PluginId {
        self.id
    }

    pub fn call(&mut self, request: &[u8]) -> Result<Vec<u8>, PluginError> {
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
