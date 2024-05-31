use std::{
    fs::File,
    io::{BufRead, BufReader},
};

const DEFAULT_READER_CAPACITY: usize = 512 * 1024; // TODO TEMP !

pub struct PluginByteSource {
    reader: BufReader<File>,
    content: Vec<u8>,
    offset: usize,
}

impl Default for PluginByteSource {
    fn default() -> Self {
        PluginByteSource::new(
            File::open("temp.dlt").expect("open"),
            DEFAULT_READER_CAPACITY,
        )
    }
}

impl PluginByteSource {
    pub fn new(input: File, total_capacity: usize) -> PluginByteSource {
        let reader = BufReader::with_capacity(total_capacity, input);
        PluginByteSource {
            reader,
            content: vec![],
            offset: 0,
        }
    }

    pub fn consume(&mut self, offset: usize) {
        if self.len() >= offset {
            self.offset += offset;
        }
    }

    pub fn reload(&mut self) -> Result<Option<ReloadInfo>, SourceError> {
        let initial_len = self.len();

        self.reader.consume(self.offset);
        self.content = self
            .reader
            .fill_buf()
            .map_err(|e| SourceError::Unrecoverable(format!("Could not fill buffer: {e}")))?
            .to_vec();
        self.offset = 0;

        let available_bytes = self.content.len();
        let newly_loaded_bytes = if available_bytes > initial_len {
            available_bytes - initial_len
        } else {
            0
        };

        if available_bytes == 0 {
            return Ok(None);
        }

        Ok(Some(ReloadInfo::new(newly_loaded_bytes, available_bytes)))
    }

    pub fn current_slice(&self) -> &[u8] {
        &self.content[self.offset..]
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.content.len() - self.offset
    }
}

#[derive(Debug)]
pub enum SourceError {
    Unrecoverable(String),
}

#[derive(Debug)]
pub struct ReloadInfo {
    pub newly_loaded_bytes: usize,
    pub available_bytes: usize,
}

impl ReloadInfo {
    pub fn new(newly_loaded_bytes: usize, available_bytes: usize) -> Self {
        Self {
            newly_loaded_bytes,
            available_bytes,
        }
    }
}
