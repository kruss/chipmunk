pub mod reader;

use reader::MinMaxBufReader;
use std::io::{BufRead, Read, Seek};

pub struct PluginByteSource<R>
where
    R: Read + Seek,
{
    reader: MinMaxBufReader<R>,
    content: Vec<u8>,
}

impl<R> PluginByteSource<R>
where
    R: Read + Seek + Unpin,
{
    pub fn new(input: R, total_capacity: usize, min_space: usize) -> PluginByteSource<R> {
        let reader = MinMaxBufReader::new(input, min_space, total_capacity);
        PluginByteSource {
            reader,
            content: vec![],
        }
    }
}

impl<R: Read + Send + Sync + Seek> PluginByteSource<R> {
    pub fn reload(&mut self) -> Result<Option<ReloadInfo>, SourceError> {
        let initial_buf_len = self.content.len();

        self.content = self
            .reader
            .fill_buf()
            .map_err(|e| SourceError::Unrecoverable(format!("Could not fill buffer: {e}")))?
            .to_vec();

        let available_bytes = self.content.len();

        let newly_loaded_bytes = if available_bytes > initial_buf_len {
            available_bytes - initial_buf_len
        } else {
            0
        };

        if available_bytes == 0 {
            return Ok(None);
        }

        Ok(Some(ReloadInfo::new(newly_loaded_bytes, available_bytes)))
    }

    pub fn current_slice(&self) -> &[u8] {
        &self.content
    }

    pub fn consume(&mut self, offset: usize) {
        self.reader.consume(offset);
    }

    pub fn len(&self) -> usize {
        self.content.len()
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

#[cfg(test)]
mod tests {
    use super::*;
    //use env_logger;

    #[test]
    fn test_binary_load() {
        //_ = env_logger::try_init();

        use std::io::{Cursor, Write};
        struct Frame {
            len: u8,
            content: Vec<u8>,
        }
        impl Frame {
            fn new(content: Vec<u8>) -> Self {
                Self {
                    len: content.len() as u8,
                    content,
                }
            }
        }

        let v: Vec<u8> = Vec::new();
        let mut buff = Cursor::new(v);

        let frame_cnt = 100;
        let total_capacity = 10;
        let min_space = 5;

        for _ in 0..frame_cnt {
            let frame = Frame::new(vec![0xA, 0xB, 0xC]);
            buff.write_all(&[frame.len]).unwrap();
            buff.write_all(&frame.content).unwrap();
        }
        buff.set_position(0);

        let total = frame_cnt * 4;
        let mut binary_source = PluginByteSource::new(buff, total_capacity, min_space);
        let mut consumed_bytes = 0usize;
        let mut consumed_msg = 0usize;
        while let Some(reload_info) = binary_source.reload().unwrap() {
            println!("=> {:?}", reload_info);
            assert!(reload_info.available_bytes >= 4);
            consumed_bytes += 4;
            consumed_msg += 1;
            binary_source.consume(4);
        }
        assert_eq!(consumed_bytes, total);
        assert_eq!(consumed_msg, frame_cnt);
    }
}
