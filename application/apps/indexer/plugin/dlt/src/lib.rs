use lazy_static::lazy_static;
use parsers::{dlt::DltParser, Error as ParserError, ParseYield, Parser};
use plugin_rpc::{dlt::*, *};
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
    static ref PARSER: Mutex<DltParser<'static>> = DltParser::default().into();
}

#[wasm_bindgen]
pub fn message(input: Vec<u8>) -> Vec<u8> {
    //print(&format!("receive request with {} bytes", input.len()));

    let request =
        rkyv::from_bytes(&input).unwrap_or_else(|error| panic("from_bytes", &error.to_string()));

    let mut parser = PARSER
        .lock()
        .unwrap_or_else(|error| panic("lock", &error.to_string()));

    let response = match request {
        PluginRequest::Plugin(DltParserRequest::Setup(DltParserSettings {
            with_storage_header
        })) => {
            print("init");
            parser.with_storage_header = with_storage_header;
            PluginResponse::Plugin(DltParserResponse::SetupDone)
        }
        PluginRequest::Plugin(DltParserRequest::Parse(DltParseInput { bytes })) => {
            let response: PluginResponse<DltParserResponse>;
            let mut results: Vec<DltParserResult> = Vec::new();
            let mut input: &[u8] = &bytes;
            loop {
                match parser.parse(input, None) {
                    Ok((rest, Some(result))) => {
                        let bytes_remaining = rest.len();
                        let message = match result {
                            ParseYield::Message(message) => {
                                //print(&format!("parse message ({} bytes remaining)", bytes_remaining));
                                Some(format!("{}", message)) // TODO
                            }
                            ParseYield::Attachment(_attachment) => {
                                print("nyi: parse attachment");
                                None // TODO
                            }
                            ParseYield::MessageAndAttachment((message, _attachment)) => {
                                print("nyi: parse attachment");
                                Some(format!("{}", message)) // TODO
                            }
                        };
                        results.push(DltParserResult::ParseOk(DltParseOutput { bytes_remaining, message }));
                        if rest.is_empty() {
                            response = PluginResponse::Plugin(DltParserResponse::ParseResult(results));
                            break;
                        } else {
                            input = rest;
                        }
                    }
                    Ok((rest, None)) => {
                        let bytes_remaining = rest.len();
                        //print(&format!("filtered message ({} bytes remaining)", bytes_remaining));
                        results.push(DltParserResult::ParseOk(DltParseOutput {
                            bytes_remaining,
                            message: None,
                        }));
                        if rest.is_empty() {
                            response = PluginResponse::Plugin(DltParserResponse::ParseResult(results));
                            break;
                        } else {
                            input = rest;
                        }
                    }
                    Err(ParserError::Incomplete) => {
                        print("parse incomplete");
                        results.push(DltParserResult::ParseIncomplete);
                        response = PluginResponse::Plugin(DltParserResponse::ParseResult(results));
                        break;
                    }
                    Err(ParserError::Eof) => {
                        print("parse eof");
                        results.push(DltParserResult::ParseEof);
                        response = PluginResponse::Plugin(DltParserResponse::ParseResult(results));
                        break;
                    }
                    Err(ParserError::Parse(error)) => {
                        //print(&format!("parse error: {}", error));
                        if results.is_empty() {
                            results.push(DltParserResult::ParseError(error));
                        }
                        response = PluginResponse::Plugin(DltParserResponse::ParseResult(results));
                        break;
                    }
                };
            }
            response
        }
        _ => {
            print(&format!("unexpected request: #{}", request));
            PluginResponse::Runtime(PluginRuntimeResponse::Error)
        }
    };
    mem::forget(input);

    let output = rkyv::to_bytes::<_, 256>(&response)
        .unwrap_or_else(|error| panic("to_bytes", &error.to_string()));

    //print(&format!("send response with {} bytes", output.len()));
    output.to_vec()
}

#[cfg(test)]
mod tests {
    use plugin_host::{wasm::WasmPluginFactory, PluginFactory};
    use plugin_rpc::{dlt::*, *};
    use std::path::PathBuf;

    fn load_plugin() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("target/wasm32-unknown-unknown/release/plugin.wasm");
        std::fs::read(path).unwrap()
    }

    #[test]
    fn test_dlt_plugin() {
        let _ = env_logger::try_init();

        let binary = load_plugin();
        let factory = WasmPluginFactory::new(binary);

        let id = 0;
        let mut proxy = factory.create(id).expect("proxy");
        assert_eq!(id, proxy.id());

        //proxy.init().expect("init");

        let dlt_bytes: Vec<u8> = vec![
            0x44, 0x4C, 0x54, 0x01, // storage header
            0x56, 0xA2, 0x91, 0x5C, 0x9C, 0x91, 0x0B, 0x00, 0x45, 0x43, 0x55, 0x31, // header
            0x3D, // header type 0b11 1101
            0x40, 0x00, 0xA2, 0x45, 0x43, 0x55, 0x31, // ecu id
            0x00, 0x00, 0x01, 0x7F, // session id
            0x00, 0x5B, 0xF7, 0x16, // timestamp
            // extended header
            0x51, // MSIN 0b101 0001 => verbose, MST log,
            0x06, // arg count
            0x56, 0x53, 0x6F, 0x6D, // app id VSom
            0x76, 0x73, 0x73, 0x64, // context id vssd
            // arguments
            0x00, 0x82, 0x00, 0x00, // type info 0b1000001000000000
            0x3A, 0x00, 0x5B, 0x33, 0x38, 0x33, 0x3A, 0x20, 0x53, 0x65, 0x72, 0x76, 0x69, 0x63,
            0x65, 0x44, 0x69, 0x73, 0x63, 0x6F, 0x76, 0x65, 0x72, 0x79, 0x55, 0x64, 0x70, 0x45,
            0x6E, 0x64, 0x70, 0x6F, 0x69, 0x6E, 0x74, 0x28, 0x31, 0x36, 0x30, 0x2E, 0x34, 0x38,
            0x2E, 0x31, 0x39, 0x39, 0x2E, 0x31, 0x30, 0x32, 0x3A, 0x35, 0x30, 0x31, 0x35, 0x32,
            0x29, 0x5D, 0x20, 0x00, 0x00, 0x82, 0x00, 0x00, // type info 0b1000001000000000
            0x0F, 0x00, // length
            0x50, 0x72, 0x6F, 0x63, 0x65, 0x73, 0x73, 0x4D, 0x65, 0x73, 0x73, 0x61, 0x67, 0x65,
            0x00, // "ProcessMessage"
            0x00, 0x82, 0x00, 0x00, // type info 0b1000001000000000
            0x02, 0x00, // length
            0x3A, 0x00, // ":"
            0x23, 0x00, 0x00, 0x00, // type info 0b10000000001000010
            0x0D, 0x01, 0x00, 0x00, 0x00, 0x82, 0x00, 0x00, 0x03, 0x00, 0x3A, 0x20, 0x00, 0x00,
            0x82, 0x00, 0x00, 0x14, 0x00, 0x31, 0x36, 0x30, 0x2E, 0x34, 0x38, 0x2E, 0x31, 0x39,
            0x39, 0x2E, 0x31, 0x36, 0x2C, 0x33, 0x30, 0x35, 0x30, 0x31, 0x00,
        ];

        let dlt_msg = "2019-03-20T02:15:50.758172000Z\u{4}ECU1\u{4}1\u{4}383\u{4}64\u{4}6027030\u{4}ECU1\u{4}VSom\u{4}vssd\u{4}LogLevel DEBUG\u{4}\u{5}[383: ServiceDiscoveryUdpEndpoint(160.48.199.102:50152)] \u{5}ProcessMessage\u{5}:\u{5}269\u{5}: \u{5}160.48.199.16,30501";

        let request = PluginRequest::Plugin(DltParserRequest::Parse(DltParseInput { bytes: dlt_bytes }));
        let input = rkyv::to_bytes::<_, 256>(&request).unwrap();
        let output = proxy.call(&input).expect("call");
        let response: PluginResponse<DltParserResponse> = rkyv::from_bytes(&output).unwrap();

        if let PluginResponse::Plugin(DltParserResponse::ParseResult(results)) = response {
            assert_eq!(1, results.len());
            let result = results.get(0).unwrap();
            if let DltParserResult::ParseOk(DltParseOutput { bytes_remaining, message }) = result {
                assert_eq!(0, *bytes_remaining);
                assert_eq!(dlt_msg, message.as_ref().unwrap());
            } else {
                panic!("invalid result");
            }
        } else {
            panic!("invalid response");
        }
    }

    #[test]
    pub fn test_dlt_requests() {
        let items = [
            PluginRequest::Runtime(PluginRuntimeRequest::Init),
            PluginRequest::Plugin(DltParserRequest::Parse(DltParseInput {
                bytes: [0, 1, 2, 3].to_vec(),
            })),
        ];

        for item in items {
            let bytes = rkyv::to_bytes::<_, 256>(&item).unwrap();
            println!("req: {}\t: {:?} => {} bytes", item, bytes, bytes.len());
            let deserialized = rkyv::from_bytes(&bytes).unwrap();
            assert_eq!(item, deserialized);
        }
    }

    #[test]
    pub fn test_dlt_responses() {
        let items = [
            PluginResponse::Runtime(PluginRuntimeResponse::Init),
            PluginResponse::Plugin(DltParserResponse::ParseResult(vec![DltParserResult::ParseEof])),
        ];

        for item in items {
            let bytes = rkyv::to_bytes::<_, 256>(&item).unwrap();
            println!("res: {}\t: {:?} => {} bytes", item, bytes, bytes.len());
            let deserialized = rkyv::from_bytes(&bytes).unwrap();
            assert_eq!(item, deserialized);
        }
    }
}
