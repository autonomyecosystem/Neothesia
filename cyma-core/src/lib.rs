//! Pure musical analysis for Neothesia Cyma.
//!
//! This crate intentionally has no dependency on wgpu, winit, or UI code.

mod chord;
mod color;
mod config;
mod harmony;
mod midi;
mod modal;
mod pitch;

pub use chord::{Chord, ChordQuality, deduce_chord_name, recognize_midi_notes};
pub use color::{MusicalColor, compose_musical_color, pitch_class_color};
pub use config::{
    CymaConfig, CymaQuality, CymaVisualization, DEFAULT_RESPONSE_TIME_MS, MAX_RESPONSE_TIME_MS,
    RESPONSE_TIME_STEP_MS,
};
pub use harmony::{HarmonicState, SmoothedHarmonicState, smoothing_alpha};
pub use midi::{MidiEvent, MidiInputError, MidiState};
pub use modal::{
    MAX_MODAL_COMPONENTS, ModalComponent, ModalField, ModalSample, PLATE_AREA_SQUARE_METERS,
    PLATE_HEIGHT_METERS, PLATE_WIDTH_METERS,
};
pub use pitch::{PITCH_CLASS_COUNT, PitchClass};
