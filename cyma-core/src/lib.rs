//! Pure musical analysis for Neothesia Cyma.
//!
//! This crate intentionally has no dependency on wgpu, winit, or UI code.

mod chord;
mod color;
mod config;
mod harmony;
mod midi;
mod pitch;

pub use chord::{Chord, ChordQuality, deduce_chord_name, recognize_midi_notes};
pub use color::{MusicalColor, compose_musical_color};
pub use config::CymaConfig;
pub use harmony::{HarmonicState, SmoothedHarmonicState, smoothing_alpha};
pub use midi::{MidiEvent, MidiInputError, MidiState};
pub use pitch::{PITCH_CLASS_COUNT, PitchClass};

/// Upper bound for the modal components produced by the first Cyma version.
pub const MAX_MODAL_COMPONENTS: usize = 12;
