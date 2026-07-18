use std::{error::Error, fmt};

use crate::{PITCH_CLASS_COUNT, PitchClass};

const MIDI_CHANNEL_COUNT: usize = 16;
const MIDI_NOTE_COUNT: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiEvent {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8 },
    SustainPedal { channel: u8, down: bool },
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiInputError {
    InvalidChannel(u8),
    InvalidNote(u8),
    DuplicateCountOverflow { channel: u8, note: u8 },
}

impl fmt::Display for MidiInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidChannel(channel) => write!(f, "invalid MIDI channel {channel}"),
            Self::InvalidNote(note) => write!(f, "invalid MIDI note {note}"),
            Self::DuplicateCountOverflow { channel, note } => {
                write!(
                    f,
                    "too many duplicate Note On events for channel {channel}, note {note}"
                )
            }
        }
    }
}

impl Error for MidiInputError {}

#[derive(Debug, Clone, Copy)]
struct NoteState {
    held_count: u16,
    sustained: bool,
    velocity: u8,
}

impl NoteState {
    const EMPTY: Self = Self {
        held_count: 0,
        sustained: false,
        velocity: 0,
    };

    const fn is_active(self) -> bool {
        self.held_count > 0 || self.sustained
    }
}

#[derive(Debug, Clone, Copy)]
struct ChannelState {
    sustain_down: bool,
    notes: [NoteState; MIDI_NOTE_COUNT],
}

impl ChannelState {
    const EMPTY: Self = Self {
        sustain_down: false,
        notes: [NoteState::EMPTY; MIDI_NOTE_COUNT],
    };
}

#[derive(Debug, Clone)]
pub struct MidiState {
    channels: [ChannelState; MIDI_CHANNEL_COUNT],
}

impl Default for MidiState {
    fn default() -> Self {
        Self {
            channels: [ChannelState::EMPTY; MIDI_CHANNEL_COUNT],
        }
    }
}

impl MidiState {
    pub fn apply(&mut self, event: MidiEvent) -> Result<bool, MidiInputError> {
        match event {
            MidiEvent::NoteOn {
                channel,
                note,
                velocity: 0,
            } => self.note_off(channel, note),
            MidiEvent::NoteOn {
                channel,
                note,
                velocity,
            } => self.note_on(channel, note, velocity),
            MidiEvent::NoteOff { channel, note } => self.note_off(channel, note),
            MidiEvent::SustainPedal { channel, down } => self.sustain(channel, down),
            MidiEvent::Reset => {
                let changed = self.active_note_count() > 0
                    || self.channels.iter().any(|channel| channel.sustain_down);
                *self = Self::default();
                Ok(changed)
            }
        }
    }

    pub fn active_note_count(&self) -> usize {
        self.channels
            .iter()
            .flat_map(|channel| channel.notes.iter())
            .filter(|note| note.is_active())
            .count()
    }

    pub fn is_note_active(&self, channel: u8, note: u8) -> Result<bool, MidiInputError> {
        let channel = self.channel(channel)?;
        let note = Self::note_index(note)?;
        Ok(channel.notes[note].is_active())
    }

    pub fn pitch_class_weights(&self) -> [f32; PITCH_CLASS_COUNT] {
        let mut weights = [0.0; PITCH_CLASS_COUNT];

        for channel in &self.channels {
            for (note, state) in channel.notes.iter().enumerate() {
                if state.is_active() {
                    let velocity = f32::from(state.velocity.max(1)) / 127.0;
                    weights[PitchClass::from_midi(note as u8).index()] += velocity;
                }
            }
        }

        normalize_weights(weights)
    }

    pub(crate) fn active_summary(&self) -> (u16, usize) {
        let mut mask = 0_u16;
        let mut count = 0_usize;

        for channel in &self.channels {
            for (note, state) in channel.notes.iter().enumerate() {
                if state.is_active() {
                    mask |= 1 << PitchClass::from_midi(note as u8).index();
                    count += 1;
                }
            }
        }

        (mask, count)
    }

    fn note_on(&mut self, channel: u8, note: u8, velocity: u8) -> Result<bool, MidiInputError> {
        let note_index = Self::note_index(note)?;
        let channel_state = self.channel_mut(channel)?;
        let note_state = &mut channel_state.notes[note_index];

        note_state.held_count = note_state
            .held_count
            .checked_add(1)
            .ok_or(MidiInputError::DuplicateCountOverflow { channel, note })?;
        note_state.velocity = velocity;

        Ok(true)
    }

    fn note_off(&mut self, channel: u8, note: u8) -> Result<bool, MidiInputError> {
        let note_index = Self::note_index(note)?;
        let channel_state = self.channel_mut(channel)?;
        let note_state = &mut channel_state.notes[note_index];

        if note_state.held_count == 0 {
            return Ok(false);
        }

        note_state.held_count -= 1;
        if note_state.held_count == 0 {
            note_state.sustained = channel_state.sustain_down;
            if !note_state.sustained {
                note_state.velocity = 0;
            }
        }

        Ok(true)
    }

    fn sustain(&mut self, channel: u8, down: bool) -> Result<bool, MidiInputError> {
        let channel_state = self.channel_mut(channel)?;
        if channel_state.sustain_down == down {
            return Ok(false);
        }

        channel_state.sustain_down = down;
        if !down {
            for note in &mut channel_state.notes {
                if note.held_count == 0 && note.sustained {
                    note.sustained = false;
                    note.velocity = 0;
                }
            }
        }

        Ok(true)
    }

    fn channel(&self, channel: u8) -> Result<&ChannelState, MidiInputError> {
        self.channels
            .get(channel as usize)
            .ok_or(MidiInputError::InvalidChannel(channel))
    }

    fn channel_mut(&mut self, channel: u8) -> Result<&mut ChannelState, MidiInputError> {
        self.channels
            .get_mut(channel as usize)
            .ok_or(MidiInputError::InvalidChannel(channel))
    }

    fn note_index(note: u8) -> Result<usize, MidiInputError> {
        if note < MIDI_NOTE_COUNT as u8 {
            Ok(note as usize)
        } else {
            Err(MidiInputError::InvalidNote(note))
        }
    }
}

pub(crate) fn normalize_weights(mut weights: [f32; PITCH_CLASS_COUNT]) -> [f32; PITCH_CLASS_COUNT] {
    for weight in &mut weights {
        if !weight.is_finite() || *weight < 0.0 {
            *weight = 0.0;
        }
    }

    let total: f32 = weights.iter().sum();
    if total > f32::EPSILON && total.is_finite() {
        for weight in &mut weights {
            *weight /= total;
        }
    }

    weights
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_on(note: u8) -> MidiEvent {
        MidiEvent::NoteOn {
            channel: 0,
            note,
            velocity: 100,
        }
    }

    fn note_off(note: u8) -> MidiEvent {
        MidiEvent::NoteOff { channel: 0, note }
    }

    #[test]
    fn handles_note_on_and_note_off() {
        let mut state = MidiState::default();
        state.apply(note_on(60)).unwrap();
        assert!(state.is_note_active(0, 60).unwrap());

        state.apply(note_off(60)).unwrap();
        assert!(!state.is_note_active(0, 60).unwrap());
    }

    #[test]
    fn velocity_zero_note_on_is_note_off() {
        let mut state = MidiState::default();
        state.apply(note_on(60)).unwrap();
        state
            .apply(MidiEvent::NoteOn {
                channel: 0,
                note: 60,
                velocity: 0,
            })
            .unwrap();

        assert!(!state.is_note_active(0, 60).unwrap());
    }

    #[test]
    fn sustain_holds_and_releases_notes() {
        let mut state = MidiState::default();
        state.apply(note_on(60)).unwrap();
        state
            .apply(MidiEvent::SustainPedal {
                channel: 0,
                down: true,
            })
            .unwrap();
        state.apply(note_off(60)).unwrap();
        assert!(state.is_note_active(0, 60).unwrap());

        state
            .apply(MidiEvent::SustainPedal {
                channel: 0,
                down: false,
            })
            .unwrap();
        assert!(!state.is_note_active(0, 60).unwrap());
    }

    #[test]
    fn duplicate_notes_require_matching_note_off_events() {
        let mut state = MidiState::default();
        state.apply(note_on(60)).unwrap();
        state.apply(note_on(60)).unwrap();
        state.apply(note_off(60)).unwrap();
        assert!(state.is_note_active(0, 60).unwrap());

        state.apply(note_off(60)).unwrap();
        assert!(!state.is_note_active(0, 60).unwrap());
    }

    #[test]
    fn normalizes_pitch_class_weights() {
        let mut state = MidiState::default();
        state.apply(note_on(60)).unwrap();
        state.apply(note_on(64)).unwrap();
        state.apply(note_on(67)).unwrap();

        let weights = state.pitch_class_weights();
        let sum: f32 = weights.iter().sum();
        assert!((sum - 1.0).abs() < 1.0e-6);
        assert!(weights.iter().all(|weight| weight.is_finite()));
    }
}
