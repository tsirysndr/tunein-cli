use futures::StreamExt;
use hyper::Body;
use std::io::{Read, Result, Seek, SeekFrom};

pub struct BodyReader {
    body: Body,
    buffer: Vec<u8>,
    position: u64,
}

impl BodyReader {
    pub fn new(body: Body) -> Self {
        Self {
            body,
            buffer: Vec::new(),
            position: 0,
        }
    }
}

impl Read for BodyReader {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut pos = 0;
        while pos < buf.len() {
            if self.buffer.is_empty() {
                let chunk = match futures::executor::block_on(self.body.next()) {
                    Some(Ok(chunk)) => chunk,
                    Some(Err(err)) => {
                        return Err(std::io::Error::new(std::io::ErrorKind::Other, err))
                    }
                    None => break,
                };
                self.buffer.extend_from_slice(&chunk);
            }
            let available = self.buffer.len();
            let remaining = buf.len() - pos;
            let to_read = usize::min(available, remaining);
            if to_read == 0 {
                break;
            }
            let end = to_read + pos;
            buf[pos..end].copy_from_slice(&self.buffer[..to_read]);
            self.buffer.drain(..to_read);
            pos = end;
        }
        Ok(pos)
    }
}

impl Seek for BodyReader {
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        match pos {
            SeekFrom::Start(n) => self.position = n,
            SeekFrom::End(n) => {
                let total_size = self.buffer.len() as u64;
                if n > 0 {
                    self.position = (total_size - n as u64) as u64;
                } else {
                    self.position = total_size - n.abs() as u64;
                }
            }
            SeekFrom::Current(n) => {
                let current_position = self.position as i64;
                let new_position = current_position + n;
                if new_position < 0 {
                    panic!("invalid seek to a negative position");
                }
                self.position = new_position as u64;
            }
        }

        if self.position < self.buffer.len() as u64 {
            Ok(self.position)
        } else {
            /*  let diff = self.position - self.buffer.len() as u64;
            let mut buf = vec![0; diff as usize];
            let mut body = self.body.write().await;
            let bytes_read = body.read_buf(&mut buf).await?;
            self.buffer.extend_from_slice(&buf[..bytes_read]);
            self.position = self.buffer.len() as u64;
            */
            Ok(self.position)
        }
    }
}
