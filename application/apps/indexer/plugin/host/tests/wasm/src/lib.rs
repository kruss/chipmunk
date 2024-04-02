use lazy_static::lazy_static;
use std::{
    ffi::{c_char, CString},
    mem,
    sync::Mutex,
};
use wasm_bindgen::prelude::*;

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
    static ref SEQ: Mutex<usize> = Mutex::new(0);
}

#[wasm_bindgen]
pub fn message(request: Vec<u8>) -> Vec<u8> {
    let mut seq = SEQ
        .lock()
        .unwrap_or_else(|error| panic("lock", &error.to_string()));
    *seq += 1;

    print(&format!(
        "receive request<{}> with {} bytes",
        seq,
        request.len()
    ));

    let response = request.clone(); // TODO
    mem::forget(request);

    print(&format!(
        "send response<{}> with {} bytes",
        seq,
        response.len()
    ));
    response.to_vec()
}
