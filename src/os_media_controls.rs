//! Operating system level media controls.

use tokio::sync::mpsc::UnboundedReceiver;

/// Operating system level media controls.
#[derive(Debug)]
pub struct OsMediaControls {
    /// Controls that interface with the OS.
    controls: souvlaki::MediaControls,
    /// Receiver for events produced by OS level interaction.
    event_receiver: UnboundedReceiver<souvlaki::MediaControlEvent>,
}

impl OsMediaControls {
    /// Create new [`OsMediaControls`].
    pub fn new() -> Result<Self, souvlaki::Error> {
        let mut controls = souvlaki::MediaControls::new(souvlaki::PlatformConfig {
            display_name: "tunein-cli",
            dbus_name: "tsirysndr.tunein-cli",
            // TODO: support windows platform
            hwnd: None,
        })?;

        let (event_sender, event_receiver) =
            tokio::sync::mpsc::unbounded_channel::<souvlaki::MediaControlEvent>();

        controls.attach(move |event| {
            event_sender.send(event).expect("receiver always alive");
        })?;

        Ok(Self {
            controls,
            event_receiver,
        })
    }

    /// Try to receive event produced by the operating system.
    ///
    /// Is [`None`] if no event is produced.
    pub fn try_recv_os_event(&mut self) -> Option<souvlaki::MediaControlEvent> {
        self.event_receiver.try_recv().ok()
    }

    /// Send the given [`Command`] to the operating system.
    pub fn send_to_os(&mut self, command: Command) -> Result<(), souvlaki::Error> {
        match command {
            Command::Play => self
                .controls
                .set_playback(souvlaki::MediaPlayback::Playing { progress: None }),
            Command::Pause => self
                .controls
                .set_playback(souvlaki::MediaPlayback::Paused { progress: None }),
            Command::SetVolume(volume) => {
                // NOTE: is supported only for MPRIS backend,
                // `souvlaki` doesn't provide a way to know this, so
                // need to use `cfg` attribute like the way it exposes
                // the platform
                #[cfg(all(
                    unix,
                    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
                ))]
                {
                    self.controls.set_volume(volume)
                }
                #[cfg(not(all(
                    unix,
                    not(any(target_os = "macos", target_os = "ios", target_os = "android"))
                )))]
                {
                    Ok(())
                }
            }
            Command::SetMetadata(metadata) => self.controls.set_metadata(metadata),
        }
    }
}

/// Commands understood by OS media controls.
#[derive(Debug, Clone)]
pub enum Command<'a> {
    Play,
    Pause,
    /// Volume must be between `0.0..=1.0`.
    SetVolume(f64),
    /// Set the [`souvlaki::MediaMetadata`].
    SetMetadata(souvlaki::MediaMetadata<'a>),
}
