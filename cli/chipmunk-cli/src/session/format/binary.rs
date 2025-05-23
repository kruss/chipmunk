//! Structures and methods to write parsed message in binary format.

use anyhow::Context;
use parsers::LogMessage;

use super::MessageFormatter;

/// Structure to write parsed message in binary format.
#[derive(Debug, Clone, Default)]
pub struct MsgBinaryFormatter {}

impl MessageFormatter for MsgBinaryFormatter {
    fn write_msg<M>(&mut self, mut writer: impl std::io::Write, msg: &M) -> anyhow::Result<()>
    where
        M: LogMessage,
    {
        msg.to_writer(&mut writer)
            .context("Error while writing binary message")?;
        Ok(())
    }
}
