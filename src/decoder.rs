use std::io::Read;
use std::sync::mpsc::Sender;
use std::time::Duration;

use anyhow::{anyhow, Error};
use rodio::Source;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::io::{MediaSourceStream, ReadOnlySource};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::equalizer::EqProcessor;

/// A chunk of decoded interleaved samples, forwarded to the visualizer.
#[derive(Debug, Clone)]
pub struct Frame {
    pub data: Vec<i16>,
    pub channels: usize,
    pub sample_rate: i32,
}

/// Decodes an Icecast/HTTP audio stream (MP3, AAC, Ogg Vorbis, FLAC, WAV, ...)
/// using symphonia and exposes it as a rodio `Source`.
pub struct StreamDecoder {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    buffer: Vec<i16>,
    offset: usize,
    channels: u16,
    sample_rate: u32,
    tx: Option<Sender<Frame>>,
    eq: EqProcessor,
}

impl StreamDecoder {
    pub fn new<R>(
        data: R,
        content_type: Option<&str>,
        tx: Option<Sender<Frame>>,
    ) -> Result<Self, Error>
    where
        R: Read + Send + Sync + 'static,
    {
        let mss = MediaSourceStream::new(Box::new(ReadOnlySource::new(data)), Default::default());

        let mut hint = Hint::new();
        if let Some(mime) = content_type {
            let mime = mime.split(';').next().unwrap_or(mime).trim();
            hint.mime_type(mime);
            if let Some(ext) = extension_for_mime(mime) {
                hint.with_extension(ext);
            }
        }

        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| anyhow!("unsupported or unrecognized stream format: {}", e))?;

        let format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow!("no supported audio track found in stream"))?;
        let track_id = track.id;

        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| anyhow!("unsupported codec: {}", e))?;

        let channels = track
            .codec_params
            .channels
            .map(|c| c.count() as u16)
            .unwrap_or(2);
        let sample_rate = track.codec_params.sample_rate.unwrap_or(44100);

        let mut this = StreamDecoder {
            format,
            decoder,
            track_id,
            buffer: Vec::new(),
            offset: 0,
            channels,
            sample_rate,
            tx,
            eq: EqProcessor::new(),
        };

        // Decode the first packet so channel count and sample rate are accurate
        // before rodio queries them.
        if !this.decode_next() {
            return Err(anyhow!("failed to decode audio stream"));
        }

        Ok(this)
    }

    /// Decode packets until one yields samples. Returns false at end of stream.
    fn decode_next(&mut self) -> bool {
        loop {
            let packet = loop {
                match self.format.next_packet() {
                    Ok(packet) if packet.track_id() == self.track_id => break packet,
                    Ok(_) => continue,
                    Err(_) => return false,
                }
            };

            match self.decoder.decode(&packet) {
                Ok(decoded) => {
                    if decoded.frames() == 0 {
                        continue;
                    }
                    let spec = *decoded.spec();
                    self.channels = spec.channels.count() as u16;
                    self.sample_rate = spec.rate;

                    let mut samples = SampleBuffer::<i16>::new(decoded.capacity() as u64, spec);
                    samples.copy_interleaved_ref(decoded);
                    self.buffer.clear();
                    self.buffer.extend_from_slice(samples.samples());
                    self.offset = 0;

                    // Route through the equalizer (no-op when disabled).
                    // Done before the visualizer send so the scope shows
                    // what is actually heard.
                    self.channels =
                        self.eq
                            .process(&mut self.buffer, self.channels, self.sample_rate);

                    if let Some(tx) = &self.tx {
                        let frame = Frame {
                            data: self.buffer.clone(),
                            channels: self.channels as usize,
                            sample_rate: self.sample_rate as i32,
                        };
                        if tx.send(frame).is_err() {
                            return false;
                        }
                    }

                    return true;
                }
                // Skip malformed packets, common at the start of live streams.
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(_) => return false,
            }
        }
    }
}

impl Source for StreamDecoder {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.buffer.len().saturating_sub(self.offset))
    }

    #[inline]
    fn channels(&self) -> u16 {
        self.channels
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

impl Iterator for StreamDecoder {
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.offset >= self.buffer.len() && !self.decode_next() {
            return None;
        }

        let v = self.buffer[self.offset];
        self.offset += 1;

        Some(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Decodes a few seconds of a live stream. Requires network access,
    /// so these tests are ignored by default; run with `cargo test -- --ignored`.
    fn decode_live_stream(url: &str) {
        let client = reqwest::blocking::Client::new();
        let response = client.get(url).send().unwrap();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(String::from);
        let mut decoder = StreamDecoder::new(response, content_type.as_deref(), None)
            .unwrap_or_else(|e| panic!("failed to open {url} ({content_type:?}): {e}"));
        println!(
            "{url}: content-type={:?} channels={} sample_rate={}",
            content_type,
            Source::channels(&decoder),
            Source::sample_rate(&decoder)
        );
        let decoded = decoder.by_ref().take(200_000).count();
        assert_eq!(decoded, 200_000, "stream {url} ended prematurely");
    }

    #[test]
    #[ignore]
    fn decodes_mp3_stream() {
        decode_live_stream("http://stream.radioparadise.com/mp3-128");
    }

    #[test]
    #[ignore]
    fn decodes_aac_stream() {
        decode_live_stream("http://stream.radioparadise.com/aac-128");
    }

    #[test]
    #[ignore]
    fn decodes_ogg_flac_stream() {
        decode_live_stream("http://stream.radioparadise.com/flacm");
    }
}

fn extension_for_mime(mime: &str) -> Option<&'static str> {
    match mime.to_ascii_lowercase().as_str() {
        "audio/mpeg" | "audio/mp3" => Some("mp3"),
        "audio/aac" | "audio/aacp" | "audio/x-aac" => Some("aac"),
        "audio/mp4" | "audio/m4a" => Some("m4a"),
        "application/ogg" | "audio/ogg" | "audio/x-ogg" => Some("ogg"),
        "audio/flac" | "audio/x-flac" => Some("flac"),
        "audio/wav" | "audio/x-wav" | "audio/wave" => Some("wav"),
        _ => None,
    }
}
