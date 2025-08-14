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
}
