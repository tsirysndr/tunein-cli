use std::time::Duration;
use std::{io::Read, sync::mpsc::Sender};

use minimp3::{Decoder, Frame};
use rodio::Source;

pub struct Mp3Decoder<R>
where
    R: Read,
{
    decoder: Decoder<R>,
    current_frame: Frame,
    current_frame_offset: usize,
    tx: Option<Sender<Frame>>,
}

impl<R> Mp3Decoder<R>
where
    R: Read,
{
    pub fn new(mut data: R, tx: Option<Sender<Frame>>) -> Result<Self, R> {
        if !is_mp3(data.by_ref()) {
            return Err(data);
        }
        let mut decoder = Decoder::new(data);
        let current_frame = decoder.next_frame().unwrap();

        Ok(Mp3Decoder {
            decoder,
            current_frame,
            current_frame_offset: 0,
            tx,
        })
    }
}

impl<R> Source for Mp3Decoder<R>
where
    R: Read,
{
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.current_frame.data.len())
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.current_frame.channels as _
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.current_frame.sample_rate as _
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl<R> Iterator for Mp3Decoder<R>
where
    R: Read,
{
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.current_frame_offset == self.current_frame.data.len() {
            match self.decoder.next_frame() {
                Ok(frame) => {
                    if let Some(tx) = &self.tx {
                        if tx.send(frame.clone()).is_err() {
                            return None;
                        }
                    }
                    self.current_frame = frame
                }
                _ => return None,
            }
            self.current_frame_offset = 0;
        }

        let v = self.current_frame.data[self.current_frame_offset];
        self.current_frame_offset += 1;

        Some(v)
    }
}

/// Returns true if the stream contains mp3 data, then resets it to where it was.
fn is_mp3<R>(mut data: R) -> bool
where
    R: Read,
{
    let mut decoder = Decoder::new(data.by_ref());
    let ok = decoder.next_frame().is_ok();
    ok
}
