use std::collections::VecDeque;
use std::io::{self, BufRead, Read};

pub struct MinMaxBufReader<R: Read> {
    inner: R,
    buf: VecDeque<u8>,
    min_size: usize,
    max_size: usize,
}

impl<R: Read> MinMaxBufReader<R> {
    pub fn new(inner: R, min_size: usize, max_size: usize) -> MinMaxBufReader<R> {
        assert!(
            min_size <= max_size,
            "min_size must be less than or equal to max_size"
        );
        MinMaxBufReader {
            inner,
            buf: VecDeque::with_capacity(max_size),
            min_size,
            max_size,
        }
    }

    fn fill_buf_min(&mut self) -> io::Result<()> {
        while self.buf.len() < self.min_size {
            if self.buf.len() >= self.max_size {
                // Buffer is already at maximum capacity
                break;
            }
            let mut temp_buf = vec![0; self.max_size - self.buf.len()];
            let read_count = self.inner.read(&mut temp_buf)?;
            if read_count == 0 {
                // End of stream
                break;
            }
            for byte in &temp_buf[..read_count] {
                self.buf.push_back(*byte);
            }
        }
        Ok(())
    }
}

impl<R: Read> Read for MinMaxBufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.fill_buf_min()?;
        let len = buf.len().min(self.buf.len());
        for i in 0..len {
            buf[i] = self.buf.pop_front().unwrap();
        }
        Ok(len)
    }
}

impl<R: Read> BufRead for MinMaxBufReader<R> {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        self.fill_buf_min()?;
        Ok(self.buf.make_contiguous())
    }

    fn consume(&mut self, amt: usize) {
        for _ in 0..amt {
            self.buf.pop_front();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_min_max_reader() {
        let data = "Hello, this is a test.".as_bytes();
        let mut reader = MinMaxBufReader::new(data, 10, 20);

        let mut buffer = String::new();
        reader.read_to_string(&mut buffer).expect("read_to_string");
        assert_eq!("Hello, this is a test.", buffer);
    }
}
