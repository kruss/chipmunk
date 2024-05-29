use serde::Serialize;
use plugin_host::{PluginError, PluginFactory, PluginId, PluginProxyObj, wasm::WasmPluginFactory};
use plugin_rpc::{*, dlt::*};
use std::{io::Write, fs, path::PathBuf, collections::VecDeque};
use parsers::{Error as ParserError, LogMessage, ParseYield, Parser};

// Plugin factory that will create a WASM dlt-parser plugin.
// TODO: Load WASM binary via configuration.
pub struct DltPluginFactory {
    factory: WasmPluginFactory
}

impl DltPluginFactory {
    pub fn new() -> Self {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../plugin/dlt/target/wasm32-unknown-unknown/release/plugin.wasm");
        let binary = fs::read(path).unwrap();
        
        DltPluginFactory { 
            factory: WasmPluginFactory::new(binary)
        }
    }
}

impl PluginFactory for DltPluginFactory {
    fn create(&self, id: PluginId) -> Result<PluginProxyObj, PluginError> {
        self.factory.create(id)
    }
}

#[derive(Debug, Serialize)]
pub struct DltProxyMessage {
    pub content: String,
}

impl DltProxyMessage {
    pub fn new(content: String) -> Self {
        DltProxyMessage {
            content
        }
    }
}

impl std::fmt::Display for DltProxyMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.content)
    }
}

impl LogMessage for DltProxyMessage {
    fn to_writer<W: Write>(&self, writer: &mut W) -> Result<usize, std::io::Error> {
        let bytes = self.content.as_bytes();
        let len = bytes.len();
        writer.write_all(bytes)?;
        Ok(len)
    }
}

pub struct DltParserProxy {
    proxy: PluginProxyObj,
    stats: DltParserStats,
    results: VecDeque<DltParserResult>
}

impl DltParserProxy {
    pub fn new(mut proxy: PluginProxyObj, with_storage_header: bool) -> Self {
        println!("\x1b[93mhost : new dlt-parser proxy<{}>\x1b[0m", proxy.id());

        let request: PluginRequest<DltParserRequest> = PluginRequest::Plugin(
            DltParserRequest::Setup(DltParserSettings {
                with_storage_header
            }));
        let request_bytes = rkyv::to_bytes::<_, 256>(&request).unwrap();

        match proxy.call(&request_bytes) {
            Ok(response_bytes) => {
                let response: PluginResponse<DltParserResponse> = rkyv::from_bytes(&response_bytes).unwrap();
                if let PluginResponse::Plugin(DltParserResponse::SetupDone) = response {
                    // nothing
                } else {
                    panic!("dlt-plugin: unexpected response: #{}", response);
                }
            }
            _ => {
                panic!("dlt-plugin: request failed");
            }
        }

        Self {
            proxy,
            stats: DltParserStats::default(),
            results: VecDeque::new()
        }
    }

    fn next_result<'b>(&mut self, input: &'b [u8]) -> Option<Result<(&'b [u8], Option<ParseYield<DltProxyMessage>>), ParserError>> {
        if let Some(result) = self.results.pop_front() {
            match result {
                DltParserResult::ParseOk(output) => {
                    let rest = if self.results.is_empty() {
                        let offset = input.len() - output.bytes_remaining;
                        &input[offset..]
                    } else {
                        input
                    };

                    if let Some(message) = output.message {
                        self.stats.messages_parsed += 1;
                        Some(Ok((rest, Some(ParseYield::Message(DltProxyMessage::new(message))))))
                    } else {
                        self.stats.messages_filtered += 1;
                        Some(Ok((rest, None)))
                    }
                }
                DltParserResult::ParseIncomplete => {
                    self.stats.parse_incomplete += 1;
                    Some(Err(ParserError::Incomplete))
                },
                DltParserResult::ParseEof => {
                    self.stats.parse_eof += 1;
                    Some(Err(ParserError::Eof))
                },
                DltParserResult::ParseError(error) => {
                    //println!("\x1b[93mhost : return error from proxy<{}> : {}\x1b[0m", self.proxy.id(), error);
                    self.stats.parse_error += 1;
                    Some(Err(ParserError::Parse(error)))
                }
            }
        } else {
            None
        }
    }
}

impl Parser<DltProxyMessage> for DltParserProxy {
    fn parse<'b>(
        &mut self,
        input: &'b [u8],
        _timestamp: Option<u64>,
    ) -> Result<(&'b [u8], Option<ParseYield<DltProxyMessage>>), ParserError> {
        self.stats.calls_total += 1;
        if let Some(result) = self.next_result(input) {
            return result;
        }

        //println!("\x1b[93mhost : send request to proxy<{}> with {} bytes\x1b[0m", self.proxy.id(), input.len());
        self.stats.calls_plugin += 1;
        let request: PluginRequest<DltParserRequest> = PluginRequest::Plugin(
            DltParserRequest::Parse(
                DltParseInput { bytes: input.to_vec() }));

        let request_bytes = rkyv::to_bytes::<_, 256>(&request).unwrap();
        match self.proxy.call(&request_bytes) {
            Ok(response_bytes) => {
                let response: PluginResponse<DltParserResponse> = rkyv::from_bytes(&response_bytes).unwrap();
                if let PluginResponse::Plugin(DltParserResponse::ParseResult(results)) = response {
                    //println!("\x1b[93mhost : received response from proxy<{}> with {} results\x1b[0m", self.proxy.id(), results.len());
                    self.stats.plugin_results += results.len();
                    self.results = VecDeque::from(results);
                    
                    if let Some(result) = self.next_result(input) {
                        result
                    } else {
                        panic!("dlt-plugin: unexpected empty result");
                    }
                } else {
                    panic!("dlt-plugin: unexpected response: #{}", response);
                }
            }
            _ => {
                panic!("dlt-plugin: request failed");
            }
        }
    }
}

impl std::ops::Drop for DltParserProxy {
    fn drop(&mut self) {
        println!("\x1b[93mhost : proxy<{}> stats : {}\x1b[0m", self.proxy.id(), self.stats);
    }
}

#[derive(Default)]
struct DltParserStats {
    pub calls_total: usize,
    pub calls_plugin: usize,
    pub plugin_results: usize,
    pub messages_parsed: usize,
    pub messages_filtered: usize,
    pub parse_incomplete: usize,
    pub parse_eof: usize,
    pub parse_error: usize,
}

impl std::fmt::Display for DltParserStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "c-fn {}, c-plg {}, p-res {}, m-ok {}, m-flt {}, p-inc {}, p-eof {}, p-err {}", 
            self.calls_total,
            self.calls_plugin,
            self.plugin_results,
            self.messages_parsed, 
            self.messages_filtered, 
            self.parse_incomplete,
            self.parse_eof,
            self.parse_error
        )
    }
}
