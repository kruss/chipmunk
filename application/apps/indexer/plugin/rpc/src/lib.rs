use rkyv::{Archive, Deserialize, Serialize};
use strum_macros::Display;

#[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
#[archive(check_bytes)]
pub enum PluginRequest<T> {
    Runtime(PluginRuntimeRequest),
    Plugin(T),
}

#[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
#[archive(check_bytes)]
pub enum PluginResponse<T> {
    Runtime(PluginRuntimeResponse),
    Plugin(T),
}

#[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
#[archive(check_bytes)]
pub enum PluginRuntimeRequest {
    Version,
    Init,
    Shutdown,
}

#[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
#[archive(check_bytes)]
pub enum PluginRuntimeResponse {
    Version,
    Init,
    Shutdown,
    Error,
}

pub mod source {
    use super::*;

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum ByteSourceRequest {
        Dummy
    }

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum ByteSourceResponse {
        Dummy
    }
}

pub mod dlt {
    use super::*;

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum DltParserRequest {
        Setup(DltParserSettings),
        Parse(DltParseInput),
    }

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[archive(check_bytes)]
    pub struct DltParserSettings {
        pub with_storage_header: bool
    }

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[archive(check_bytes)]
    pub struct DltParseInput {
        pub bytes: Vec<u8>
    }

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum DltParserResponse {
        SetupDone,
        ParseResult(Vec<DltParserResult>)
    }

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum DltParserResult {
        ParseOk(DltParseOutput),
        ParseIncomplete,
        ParseEof,
        ParseError(String)
    }

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[archive(check_bytes)]
    pub struct DltParseOutput {
        pub bytes_remaining: usize,
        pub message: Option<String>,
    }
}