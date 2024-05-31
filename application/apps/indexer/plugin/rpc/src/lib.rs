use rkyv::{Archive, Deserialize, Serialize};
use strum_macros::Display;

#[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
#[archive(check_bytes)]
pub enum PluginRpc<T> {
    Request(T),
    Response(T),
    Unexpected,
}

pub mod source {
    use super::*;

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum ByteSourceRpc {
        Setup(SourceSettings),
        SetupDone,
        Reload(usize),
        ReloadResult(ReloadResult)
    }

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[archive(check_bytes)]
    pub struct SourceSettings {
        pub input_path: String,
        pub total_capacity: usize,
        pub buffer_min: usize
    }

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum ReloadResult {
        ReloadOk(ReloadOutput),
        ReloadEof,
        ReloadError(String)
    }

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[archive(check_bytes)]
    pub struct ReloadOutput {
        pub newly_loaded_bytes: usize,
        pub available_bytes: usize,
        pub skipped_bytes: usize,
        pub bytes: Vec<u8>,
    }
}

pub mod dlt {
    use super::*;

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum DltParserRpc {
        Setup(ParserSettings),
        SetupDone,
        Parse(ParseInput),
        ParseResult(Vec<ParserResult>)
    }

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[archive(check_bytes)]
    pub struct ParserSettings {
        pub with_storage_header: bool
    }

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[archive(check_bytes)]
    pub struct ParseInput {
        pub bytes: Vec<u8>
    }

    #[derive(Archive, Serialize, Deserialize, Debug, Display, PartialEq)]
    #[archive(check_bytes)]
    pub enum ParserResult {
        ParseOk(ParseOutput),
        ParseIncomplete,
        ParseEof,
        ParseError(String)
    }

    #[derive(Archive, Serialize, Deserialize, Debug, PartialEq)]
    #[archive(check_bytes)]
    pub struct ParseOutput {
        pub bytes_remaining: usize,
        pub message: Option<String>,
    }
}