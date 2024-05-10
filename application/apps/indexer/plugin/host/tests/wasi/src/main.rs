#![no_main]

use lazy_static::lazy_static;
use std::{
    ffi::{c_char, CString},
    fs,
    fs::File,
    io::{prelude::*, BufReader},
    sync::Mutex,
};

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

#[repr(C)]
pub struct Response(*mut u8, u32);

#[allow(clippy::missing_safety_doc)]
#[no_mangle]
pub unsafe extern "C" fn message(ptr: *const u8, len: u32) -> Response {
    let mut seq = SEQ
        .lock()
        .unwrap_or_else(|error| panic("lock", &error.to_string()));
    *seq += 1;

    let request = unsafe { std::slice::from_raw_parts(ptr, len.try_into().unwrap()) };

    print(&format!(
        "receive request<{}> with {} bytes : {:?}",
        seq,
        request.len(),
        request
    ));

    test_file_io();
    let mut response = request.to_vec(); // TODO

    print(&format!(
        "send response<{}> with {} bytes : {:?}",
        seq,
        response.len(),
        response
    ));

    let ptr = response.as_mut_ptr();
    let len = response.len();
    std::mem::forget(response);

    Response(ptr, len as u32)
}

fn test_file_io() {
    let mut file = fs::File::create("/wasi.txt").unwrap();
    writeln!(file, "Hello WASI!").unwrap();
    let file = File::open("/wasi.txt").unwrap_or_else(|error| panic("open", &error.to_string()));
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line.unwrap_or_else(|error| panic("line", &error.to_string()));
        print(&line);
    }
}
