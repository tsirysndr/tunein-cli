use crate::music::Note;

/// a simple oscilloscope/vectorscope for your terminal
#[derive(Debug)]
pub struct ScopeArgs {
    pub opts: SourceOptions,
    pub ui: UiOptions,
}

#[derive(Debug, Clone)]
pub struct UiOptions {
    /// floating point vertical scale, from 0 to 1
    pub scale: f32,

    /// use vintage looking scatter mode instead of line mode
    pub scatter: bool,

    /// don't draw reference line
    pub no_reference: bool,

    /// hide UI and only draw waveforms
    pub no_ui: bool,

    /// don't use braille dots for drawing lines
    pub no_braille: bool,
}

#[derive(Debug, Clone)]
pub struct SourceOptions {
    /// number of channels to open
    pub channels: usize,

    /// size of audio buffer, and width of scope
    pub buffer: u32,

    /// sample rate to use
    pub sample_rate: u32,

    /// tune buffer size to be in tune with given note (overrides buffer option)
    pub tune: Option<String>,
}

// TODO its convenient to keep this here but it's not really the best place...
impl SourceOptions {
    pub fn tune(&mut self) {
        if let Some(txt) = &self.tune {
            // TODO make it less jank
            if let Ok(note) = txt.parse::<Note>() {
                self.buffer = note.tune_buffer_size(self.sample_rate);
                while self.buffer % (self.channels as u32 * 2) != 0 {
                    // TODO customizable bit depth
                    self.buffer += 1; // TODO jank but otherwise it doesn't align
                }
            } else {
                eprintln!("[!] Unrecognized note '{}', ignoring option", txt);
            }
        }
    }
}
