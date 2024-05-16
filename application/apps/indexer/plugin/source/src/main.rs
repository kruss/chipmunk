#![no_main]

use lazy_static::lazy_static;
use plugin_rpc::{source::*, *};
use std::{
    ffi::{c_char, CString},
    mem,
    sync::Mutex,
};

use plugin::{PluginByteSource, SourceError};

#[link(wasm_import_module = "host")]
extern "C" {
    fn host_print(ptr: *mut c_char, len: u32);
}

fn print(what: &str) {
    let ptr = CString::new(what).unwrap().into_raw();
    let len = what.len();
    unsafe {
        host_print(ptr, len as u32);
    }
}

fn panic(what: &str, error: &str) -> ! {
    print(&format!("'{}' failed: {:?}", what, error));
    panic!();
}

lazy_static! {
    static ref SOURCE: Mutex<PluginByteSource> = PluginByteSource::default().into();
}

#[repr(C)]
pub struct Response(*mut u8, u32);

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn message(ptr: *const u8, len: u32) -> Response {
    let input = unsafe { std::slice::from_raw_parts(ptr, len.try_into().unwrap()) };
    //print(&format!("receive request with {} bytes", input.len()));

    let request =
        rkyv::from_bytes(&input).unwrap_or_else(|error| panic("from_bytes", &error.to_string()));

    let mut source = SOURCE
        .lock()
        .unwrap_or_else(|error| panic("lock", &error.to_string()));

    let response = match request {
        PluginRequest::Plugin(ByteSourceRequest::Setup(ByteSourceSettings {
            input_path,
            ..
            //total_capacity,
            //buffer_min,
        })) => {
            print(&format!("init source: {}", input_path));
            // TODO
            PluginResponse::Plugin(ByteSourceResponse::SetupDone)
        }
        PluginRequest::Plugin(ByteSourceRequest::Reload(offset)) => {
            //print(&format!("consume: {}", offset));
            source.consume(offset);

            match source.reload() {
                Ok(None) => {
                    print("reload eof");
                    PluginResponse::Plugin(ByteSourceResponse::ReloadResult(
                        SourceReloadResult::ReloadEof
                    ))
                },
                Ok(Some(reload)) => {
                    let slice = source.current_slice();
                    /*
                    print(&format!("reload new: {}, avl: {}, bytes: {}", 
                        reload.newly_loaded_bytes, 
                        reload.available_bytes,
                        slice.len()
                    ));
                     */
                    PluginResponse::Plugin(ByteSourceResponse::ReloadResult(
                        SourceReloadResult::ReloadOk(SourceReloadOutput {
                            newly_loaded_bytes: reload.newly_loaded_bytes,
                            available_bytes: reload.available_bytes,
                            skipped_bytes: 0,
                            bytes: slice.to_vec()
                        }),
                    ))
                },
                Err(SourceError::Unrecoverable(error)) => {
                    print(&format!("source error: {}", error));
                    PluginResponse::Plugin(ByteSourceResponse::ReloadResult(
                        SourceReloadResult::ReloadError(error),
                    ))
                }
            }
        }
        _ => {
            print(&format!("unexpected request: #{}", request));
            PluginResponse::Runtime(PluginRuntimeResponse::Error)
        }
    };

    let mut output = rkyv::to_bytes::<_, 256>(&response)
        .unwrap_or_else(|error| panic("to_bytes", &error.to_string()));
    //print(&format!("send response with {} bytes", output.len()));

    let ptr = output.as_mut_ptr();
    let len = output.len();
    mem::forget(output);

    Response(ptr, len as u32)
}

/*
#[cfg(test)]
mod tests {
    use plugin_host::{wasi::WasiPluginFactory, PluginFactory};
    use plugin_rpc::{source::*, *};
    use std::path::PathBuf;

    fn load_plugin() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("target/wasm32-wasi/release/plugin.wasm");
        std::fs::read(path).unwrap()
    }

    #[test]
    fn test_source_plugin() {
        let _ = env_logger::try_init();

        let binary = load_plugin();
        let factory = WasiPluginFactory::new(binary);

        let id = 0;
        let mut proxy = factory.create(id).expect("proxy");
        assert_eq!(id, proxy.id());

        // TODO
    }
}
 */
