use std::time::Duration;

use cyma_core::{
    HarmonicState, MidiEvent as CymaMidiEvent, MidiInputError, MidiState, ModalField,
    SmoothedHarmonicState,
};
use midi_file::midly::MidiMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CymaMidiSource {
    User,
    File,
}

#[derive(Default)]
pub struct CymaState {
    runtime: Option<CymaRuntime>,
}

impl CymaState {
    pub fn new(enabled: bool) -> Self {
        Self {
            runtime: enabled.then(CymaRuntime::default),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.runtime.is_some()
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        match (enabled, self.runtime.is_some()) {
            (true, false) => self.runtime = Some(CymaRuntime::default()),
            (false, true) => self.runtime = None,
            _ => {}
        }
    }

    pub fn observe_midi_event(
        &mut self,
        source: CymaMidiSource,
        channel: u8,
        message: &MidiMessage,
    ) -> Result<bool, MidiInputError> {
        let Some(runtime) = self.runtime.as_mut() else {
            return Ok(false);
        };

        runtime.observe_midi_event(source, channel, message)
    }

    pub fn reset_source(&mut self, source: CymaMidiSource) -> Result<bool, MidiInputError> {
        let Some(runtime) = self.runtime.as_mut() else {
            return Ok(false);
        };

        runtime.reset_source(source)
    }

    pub fn update(&mut self, delta: Duration, response_seconds: f32) {
        let Some(runtime) = self.runtime.as_mut() else {
            return;
        };

        runtime.refresh_target_if_dirty();
        runtime
            .smoothed
            .advance(&runtime.target, delta.as_secs_f32(), response_seconds);
        runtime.modal = ModalField::from_harmonic(runtime.smoothed.current());
    }

    pub fn harmonic_state(&self) -> Option<&HarmonicState> {
        self.runtime
            .as_ref()
            .map(|runtime| runtime.smoothed.current())
    }

    pub fn modal_field(&self) -> Option<&ModalField> {
        self.runtime.as_ref().map(|runtime| &runtime.modal)
    }
}

#[derive(Default)]
struct CymaRuntime {
    user: MidiState,
    file: MidiState,
    target: HarmonicState,
    smoothed: SmoothedHarmonicState,
    modal: ModalField,
    dirty: bool,
}

impl CymaRuntime {
    fn observe_midi_event(
        &mut self,
        source: CymaMidiSource,
        channel: u8,
        message: &MidiMessage,
    ) -> Result<bool, MidiInputError> {
        let Some(event) = to_cyma_midi_event(channel, message) else {
            return Ok(false);
        };

        let changed = self.source_mut(source).apply(event)?;
        if changed {
            self.dirty = true;
        }

        Ok(changed)
    }

    fn reset_source(&mut self, source: CymaMidiSource) -> Result<bool, MidiInputError> {
        let changed = self.source_mut(source).apply(CymaMidiEvent::Reset)?;
        if changed {
            self.dirty = true;
        }

        Ok(changed)
    }

    fn source_mut(&mut self, source: CymaMidiSource) -> &mut MidiState {
        match source {
            CymaMidiSource::User => &mut self.user,
            CymaMidiSource::File => &mut self.file,
        }
    }

    fn refresh_target_if_dirty(&mut self) {
        if self.dirty {
            self.target = HarmonicState::from_midi_states(&[&self.user, &self.file]);
            self.dirty = false;
        }
    }
}

fn to_cyma_midi_event(channel: u8, message: &MidiMessage) -> Option<CymaMidiEvent> {
    if channel == 9 {
        return None;
    }

    match message {
        MidiMessage::NoteOn { key, vel } => Some(CymaMidiEvent::NoteOn {
            channel,
            note: key.as_int(),
            velocity: vel.as_int(),
        }),
        MidiMessage::NoteOff { key, .. } => Some(CymaMidiEvent::NoteOff {
            channel,
            note: key.as_int(),
        }),
        MidiMessage::Controller { controller, value } if controller.as_int() == 64 => {
            Some(CymaMidiEvent::SustainPedal {
                channel,
                down: value.as_int() >= 64,
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_on(note: u8) -> MidiMessage {
        MidiMessage::NoteOn {
            key: note.into(),
            vel: 100.into(),
        }
    }

    fn note_off(note: u8) -> MidiMessage {
        MidiMessage::NoteOff {
            key: note.into(),
            vel: 0.into(),
        }
    }

    #[test]
    fn disabled_state_does_not_allocate_runtime_or_observe_events() {
        let mut state = CymaState::new(false);
        assert!(!state.is_enabled());
        assert!(
            !state
                .observe_midi_event(CymaMidiSource::User, 0, &note_on(60))
                .unwrap()
        );
        assert!(state.harmonic_state().is_none());
    }

    #[test]
    fn enabling_starts_from_an_empty_state() {
        let mut state = CymaState::new(false);
        state.set_enabled(true);
        state
            .observe_midi_event(CymaMidiSource::User, 0, &note_on(60))
            .unwrap();
        state.update(Duration::from_millis(16), 0.0);

        let harmonic = state.harmonic_state().unwrap();
        assert_eq!(harmonic.chord.to_string(), "C");
        assert_eq!(harmonic.active_note_count, 1);
        assert_eq!(state.modal_field().unwrap().component_count(), 1);
    }

    #[test]
    fn resetting_file_notes_preserves_user_notes() {
        let mut state = CymaState::new(true);
        state
            .observe_midi_event(CymaMidiSource::User, 0, &note_on(60))
            .unwrap();
        for note in [64, 67] {
            state
                .observe_midi_event(CymaMidiSource::File, 0, &note_on(note))
                .unwrap();
        }
        state.update(Duration::from_millis(16), 0.0);
        assert_eq!(state.harmonic_state().unwrap().chord.to_string(), "CM");

        state.reset_source(CymaMidiSource::File).unwrap();
        state.update(Duration::from_millis(16), 0.0);
        let harmonic = state.harmonic_state().unwrap();
        assert_eq!(harmonic.chord.to_string(), "C");
        assert_eq!(harmonic.active_note_count, 1);
    }

    #[test]
    fn maps_sustain_controller_without_losing_note_state() {
        let mut state = CymaState::new(true);
        state
            .observe_midi_event(CymaMidiSource::User, 0, &note_on(60))
            .unwrap();
        state
            .observe_midi_event(
                CymaMidiSource::User,
                0,
                &MidiMessage::Controller {
                    controller: 64.into(),
                    value: 127.into(),
                },
            )
            .unwrap();
        state
            .observe_midi_event(CymaMidiSource::User, 0, &note_off(60))
            .unwrap();
        state.update(Duration::from_millis(16), 0.0);
        assert_eq!(state.harmonic_state().unwrap().active_note_count, 1);

        state
            .observe_midi_event(
                CymaMidiSource::User,
                0,
                &MidiMessage::Controller {
                    controller: 64.into(),
                    value: 0.into(),
                },
            )
            .unwrap();
        state.update(Duration::from_millis(16), 0.0);
        assert_eq!(state.harmonic_state().unwrap().active_note_count, 0);
    }

    #[test]
    fn percussion_channel_is_not_used_for_harmonic_analysis() {
        let mut state = CymaState::new(true);
        assert!(
            !state
                .observe_midi_event(CymaMidiSource::User, 9, &note_on(60))
                .unwrap()
        );
        state.update(Duration::from_millis(16), 0.0);
        assert_eq!(state.harmonic_state().unwrap().active_note_count, 0);
    }
}
