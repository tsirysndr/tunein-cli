//! Process-wide 10-band equalizer backed by the Rockbox DSP.
//!
//! The UI mutates one global [`Equalizer`]; every [`crate::decoder::StreamDecoder`]
//! owns an [`EqProcessor`] that watches the global's version counter and
//! routes decoded packets through `rockbox_dsp::Dsp` when the EQ is enabled.

use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use rockbox_dsp::{eq_band_setting, Dsp, EQ_NUM_BANDS};

use crate::settings::{EqBand, Settings, EQ_BANDS};

/// Shared equalizer state. Cheap to read from the audio thread: the
/// enabled flag and version counter are atomics, so the per-packet hot
/// path only takes the bands lock after an actual change.
pub struct Equalizer {
    enabled: AtomicBool,
    version: AtomicU64,
    bands: Mutex<Vec<EqBand>>,
    /// Bass/treble shelf gains in whole dB. Like Rockbox, the tone stage
    /// is independent of the EQ on/off switch: 0 dB means off.
    bass: AtomicI32,
    treble: AtomicI32,
    /// Shelf cutoffs in Hz, 0 = Rockbox defaults (200 / 3500). Only
    /// settable via the settings file.
    bass_cutoff: AtomicI32,
    treble_cutoff: AtomicI32,
}

static GLOBAL: OnceLock<Equalizer> = OnceLock::new();

impl Equalizer {
    /// The process-wide equalizer, seeded from the settings file on first use.
    pub fn global() -> &'static Equalizer {
        GLOBAL.get_or_init(|| {
            let settings = Settings::load();
            Equalizer {
                enabled: AtomicBool::new(settings.eq_enabled),
                version: AtomicU64::new(0),
                bands: Mutex::new(settings.eq_band_settings),
                bass: AtomicI32::new(settings.bass),
                treble: AtomicI32::new(settings.treble),
                bass_cutoff: AtomicI32::new(settings.bass_cutoff),
                treble_cutoff: AtomicI32::new(settings.treble_cutoff),
            }
        })
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    /// Whether any DSP stage has work to do: the band EQ when enabled, or
    /// the tone shelves whenever their gain is nonzero (Rockbox semantics —
    /// bass/treble are independent of the EQ switch).
    pub fn is_active(&self) -> bool {
        self.is_enabled() || self.bass() != 0 || self.treble() != 0
    }

    pub fn bass(&self) -> i32 {
        self.bass.load(Ordering::Relaxed)
    }

    pub fn treble(&self) -> i32 {
        self.treble.load(Ordering::Relaxed)
    }

    /// Adjust the bass shelf gain by `delta_db`, clamped to ±24 dB.
    pub fn adjust_bass(&self, delta_db: i32) {
        let new = (self.bass() + delta_db).clamp(-24, 24);
        self.bass.store(new, Ordering::Relaxed);
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    /// Adjust the treble shelf gain by `delta_db`, clamped to ±24 dB.
    pub fn adjust_treble(&self, delta_db: i32) {
        let new = (self.treble() + delta_db).clamp(-24, 24);
        self.treble.store(new, Ordering::Relaxed);
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }

    pub fn bands(&self) -> Vec<EqBand> {
        self.bands.lock().unwrap().clone()
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    /// Replace the whole band list (e.g. when loading a preset).
    pub fn set_bands(&self, new_bands: Vec<EqBand>) {
        *self.bands.lock().unwrap() = new_bands;
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    /// Adjust one band's gain by `delta_tenths_db`, clamped to ±24 dB
    /// (Rockbox's own limit). Returns the new gain in tenths of dB.
    pub fn adjust_band_gain(&self, band: usize, delta_tenths_db: i32) -> i32 {
        let mut bands = self.bands.lock().unwrap();
        let gain = if let Some(b) = bands.get_mut(band) {
            b.gain = (b.gain + delta_tenths_db).clamp(-240, 240);
            b.gain
        } else {
            0
        };
        drop(bands);
        self.version.fetch_add(1, Ordering::Relaxed);
        gain
    }

    /// Reset every band's gain and the tone shelves to 0 dB (cutoffs and
    /// Q are kept).
    pub fn reset_gains(&self) {
        let mut bands = self.bands.lock().unwrap();
        for b in bands.iter_mut() {
            b.gain = 0;
        }
        drop(bands);
        self.bass.store(0, Ordering::Relaxed);
        self.treble.store(0, Ordering::Relaxed);
        self.version.fetch_add(1, Ordering::Relaxed);
    }

    /// Persist the current state to the settings file.
    pub fn save(&self) {
        let settings = Settings {
            eq_enabled: self.is_enabled(),
            eq_band_settings: self.bands(),
            bass: self.bass(),
            treble: self.treble(),
            bass_cutoff: self.bass_cutoff.load(Ordering::Relaxed),
            treble_cutoff: self.treble_cutoff.load(Ordering::Relaxed),
        };
        if let Err(err) = settings.save() {
            eprintln!("warning: failed to save settings: {}", err);
        }
    }
}

/// `rockbox_dsp::Dsp` wraps a process-wide singleton and holds a raw
/// pointer, so it is not `Send`. Decoding happens on whichever thread
/// rodio pulls samples from, so we need to move it there; `DSP_CALL`
/// serializes every use across threads, which makes that sound.
struct SendDsp(Dsp);
unsafe impl Send for SendDsp {}

/// Serializes all access to the Rockbox DSP singleton. Two decoders can
/// briefly coexist during a station switch; without this their configure
/// and process calls could interleave on the shared C state.
static DSP_CALL: Mutex<()> = Mutex::new(());

/// Per-decoder handle that lazily instantiates the DSP and keeps it in
/// sync with the global [`Equalizer`].
pub struct EqProcessor {
    dsp: Option<SendDsp>,
    dsp_rate: u32,
    applied_version: u64,
    out: Vec<i16>,
}

impl EqProcessor {
    pub fn new() -> Self {
        Self {
            dsp: None,
            dsp_rate: 0,
            applied_version: u64::MAX,
            out: Vec::new(),
        }
    }

    /// Run one decoded packet of interleaved samples through the EQ if it
    /// is enabled. `buffer` is replaced with the processed samples and the
    /// (possibly upmixed) channel count is returned; mono input comes back
    /// stereo because the Rockbox pipeline is configured for interleaved
    /// stereo.
    pub fn process(&mut self, buffer: &mut Vec<i16>, channels: u16, sample_rate: u32) -> u16 {
        let eq = Equalizer::global();
        if !eq.is_active() || buffer.is_empty() || channels == 0 || channels > 2 {
            return channels;
        }

        let _guard = DSP_CALL.lock().unwrap();

        // (Re)create the DSP when first needed or when the stream's sample
        // rate changed; creation resets the singleton config, so the
        // settings must be reapplied afterwards.
        let version = eq.version();
        if self.dsp.is_none() || self.dsp_rate != sample_rate {
            let mut dsp = Dsp::new(sample_rate);
            apply_settings(&mut dsp, eq);
            self.dsp = Some(SendDsp(dsp));
            self.dsp_rate = sample_rate;
            self.applied_version = version;
        } else if self.applied_version != version {
            let dsp = &mut self.dsp.as_mut().unwrap().0;
            apply_settings(dsp, eq);
            self.applied_version = version;
        }

        // The pipeline expects interleaved stereo; duplicate mono samples.
        if channels == 1 {
            let mono = std::mem::take(buffer);
            buffer.reserve(mono.len() * 2);
            for s in mono {
                buffer.push(s);
                buffer.push(s);
            }
        }

        let dsp = &mut self.dsp.as_mut().unwrap().0;
        self.out.clear();
        dsp.process(buffer, &mut self.out);
        std::mem::swap(buffer, &mut self.out);

        2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::default_eq_band_settings;

    fn sine_stereo(freq_hz: f64, rate: u32, frames: usize) -> Vec<i16> {
        let mut pcm = Vec::with_capacity(frames * 2);
        for n in 0..frames {
            let t = n as f64 / rate as f64;
            let s = (0.25 * (2.0 * std::f64::consts::PI * freq_hz * t).sin() * 32767.0) as i16;
            pcm.push(s);
            pcm.push(s);
        }
        pcm
    }

    fn rms(pcm: &[i16]) -> f64 {
        let sum: f64 = pcm.iter().map(|&s| (s as f64) * (s as f64)).sum();
        (sum / pcm.len() as f64).sqrt()
    }

    /// One test rather than several: it mutates the process-wide equalizer,
    /// so splitting it up would race under the parallel test runner.
    #[test]
    fn processor_follows_global_state() {
        let eq = Equalizer::global();
        eq.set_bands(default_eq_band_settings());
        eq.reset_gains();
        let mut processor = EqProcessor::new();

        // Disabled and tone flat → buffer passes through untouched.
        eq.set_enabled(false);
        let original = sine_stereo(1000.0, 44100, 44100);
        let mut buf = original.clone();
        assert_eq!(processor.process(&mut buf, 2, 44100), 2);
        assert_eq!(buf, original);

        // −12 dB on the 1 kHz band should clearly attenuate a 1 kHz tone.
        eq.set_enabled(true);
        assert_eq!(eq.bands()[5].cutoff, 1000);
        eq.adjust_band_gain(5, -120);
        let mut cut = original.clone();
        assert_eq!(processor.process(&mut cut, 2, 44100), 2);
        let ratio = rms(&original) / rms(&cut);
        assert!(
            ratio > 2.0 && ratio < 8.0,
            "expected ~4x attenuation at 1 kHz, got {ratio:.2}x"
        );

        // Mono input comes back upmixed to interleaved stereo.
        let mono: Vec<i16> = original.iter().step_by(2).cloned().collect();
        let mut mono_buf = mono.clone();
        assert_eq!(processor.process(&mut mono_buf, 1, 44100), 2);
        assert_eq!(mono_buf.len(), mono.len() * 2);

        // Bass shelf works even with the band EQ switched off: a −12 dB
        // bass cut should clearly attenuate a 100 Hz tone.
        eq.reset_gains();
        eq.set_enabled(false);
        eq.adjust_bass(-12);
        assert!(eq.is_active());
        let low = sine_stereo(100.0, 44100, 44100);
        let mut low_cut = low.clone();
        assert_eq!(processor.process(&mut low_cut, 2, 44100), 2);
        let ratio = rms(&low) / rms(&low_cut);
        assert!(
            ratio > 2.0 && ratio < 8.0,
            "expected ~4x bass attenuation at 100 Hz, got {ratio:.2}x"
        );

        eq.reset_gains();
        eq.set_enabled(false);
    }
}

fn apply_settings(dsp: &mut Dsp, eq: &Equalizer) {
    for (i, band) in eq
        .bands()
        .iter()
        .take(EQ_NUM_BANDS.min(EQ_BANDS))
        .enumerate()
    {
        dsp.set_eq_band_raw(
            i,
            eq_band_setting {
                cutoff: band.cutoff,
                q: band.q,
                gain: band.gain,
            },
        );
    }
    dsp.eq_enable(eq.is_enabled());

    // Cutoffs must be set BEFORE gains — set_tone runs the prescale step
    // that recomputes the shelf coefficients from the active cutoff.
    dsp.set_tone_cutoffs(
        eq.bass_cutoff.load(Ordering::Relaxed),
        eq.treble_cutoff.load(Ordering::Relaxed),
    );
    dsp.set_tone(eq.bass(), eq.treble());
}
