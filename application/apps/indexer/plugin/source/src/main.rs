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
        rkyv::from_bytes(input).unwrap_or_else(|error| panic("from_bytes", &error.to_string()));

    let mut source = SOURCE
        .lock()
        .unwrap_or_else(|error| panic("lock", &error.to_string()));

    let response = match request {
        PluginRpc::Request(ByteSourceRpc::Setup(SourceSettings {
            input_path,
            ..
            //total_capacity,
            //buffer_min,
        })) => {
            print(&format!("init source: {}", input_path));
            // TODO
            PluginRpc::Response(ByteSourceRpc::SetupDone)
        }
        PluginRpc::Request(ByteSourceRpc::Reload(offset)) => {
            //print(&format!("consume: {}", offset));
            source.consume(offset);

            match source.reload() {
                Ok(None) => {
                    print("reload eof");
                    PluginRpc::Response(ByteSourceRpc::ReloadResult(
                        ReloadResult::ReloadEof
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
                    PluginRpc::Response(ByteSourceRpc::ReloadResult(
                        ReloadResult::ReloadOk(ReloadOutput {
                            newly_loaded_bytes: reload.newly_loaded_bytes,
                            available_bytes: reload.available_bytes,
                            skipped_bytes: 0,
                            bytes: slice.to_vec()
                        }),
                    ))
                },
                Err(SourceError::Unrecoverable(error)) => {
                    print(&format!("source error: {}", error));
                    PluginRpc::Response(ByteSourceRpc::ReloadResult(
                        ReloadResult::ReloadError(error),
                    ))
                }
            }
        }
        _ => {
            print(&format!("unexpected request: #{}", request));
            PluginRpc::Unexpected
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
