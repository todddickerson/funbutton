use anyhow::{anyhow, Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use parking_lot::Mutex;
use std::io::Cursor;
use std::sync::Arc;

/// Audio recorder backed by cpal. Captures to in-memory f32 mono samples
/// at the device's native rate; encodes to WAV PCM-16 on stop.
pub struct Recorder {
    inner: Arc<Inner>,
    stream: Option<Stream>,
}

struct Inner {
    samples: Mutex<Vec<f32>>,
    sample_rate: Mutex<u32>,
    channels: Mutex<u16>,
}

// SAFETY: we never share the Stream across threads ourselves; cpal's Stream is !Send,
// so we hold it only on the recorder thread. The Inner is Send/Sync via Mutex.
unsafe impl Send for Recorder {}

impl Recorder {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Inner {
                samples: Mutex::new(Vec::new()),
                sample_rate: Mutex::new(16_000),
                channels: Mutex::new(1),
            }),
            stream: None,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        if self.stream.is_some() {
            return Err(anyhow!("recorder already started"));
        }
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("no default input device available")?;
        let supported = device
            .default_input_config()
            .context("no default input config")?;
        let sample_format = supported.sample_format();
        let config: cpal::StreamConfig = supported.clone().into();

        *self.inner.sample_rate.lock() = config.sample_rate;
        *self.inner.channels.lock() = config.channels;
        self.inner.samples.lock().clear();

        let inner = Arc::clone(&self.inner);
        let inner_err = Arc::clone(&self.inner);
        let err_fn = move |e| {
            log::error!("audio stream error: {e}");
            let _ = &inner_err;
        };

        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    let mut buf = inner.samples.lock();
                    buf.extend_from_slice(data);
                },
                err_fn,
                None,
            )?,
            SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    let mut buf = inner.samples.lock();
                    buf.extend(data.iter().map(|s| *s as f32 / i16::MAX as f32));
                },
                err_fn,
                None,
            )?,
            SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    let mut buf = inner.samples.lock();
                    buf.extend(data.iter().map(|s| (*s as f32 - 32768.0) / 32768.0));
                },
                err_fn,
                None,
            )?,
            other => return Err(anyhow!("unsupported sample format: {:?}", other)),
        };
        stream.play()?;
        self.stream = Some(stream);
        Ok(())
    }

    pub fn stop_and_encode_wav(&mut self) -> Result<Vec<u8>> {
        let stream = self
            .stream
            .take()
            .ok_or_else(|| anyhow!("recorder not started"))?;
        drop(stream); // stops the stream
        let samples = self.inner.samples.lock().clone();
        let sample_rate = *self.inner.sample_rate.lock();
        let channels = *self.inner.channels.lock();
        encode_wav_mono16(&samples, sample_rate, channels)
    }

    pub fn sample_count(&self) -> usize {
        self.inner.samples.lock().len()
    }

    pub fn duration_ms(&self) -> u64 {
        let samples = self.inner.samples.lock().len();
        let sr = *self.inner.sample_rate.lock() as u64;
        let ch = *self.inner.channels.lock() as u64;
        if sr == 0 || ch == 0 { return 0; }
        (samples as u64) * 1000 / (sr * ch)
    }
}

/// Encode interleaved f32 samples as 16-bit PCM mono WAV.
/// If input is multi-channel, average channels to mono.
fn encode_wav_mono16(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>> {
    let mut buf: Vec<u8> = Vec::with_capacity(samples.len() * 2 + 64);
    {
        let cursor = Cursor::new(&mut buf);
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(cursor, spec)?;
        if channels == 1 {
            for &s in samples {
                let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                writer.write_sample(v)?;
            }
        } else {
            let ch = channels as usize;
            for frame in samples.chunks_exact(ch) {
                let mix: f32 = frame.iter().sum::<f32>() / ch as f32;
                let v = (mix.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
                writer.write_sample(v)?;
            }
        }
        writer.finalize()?;
    }
    Ok(buf)
}
