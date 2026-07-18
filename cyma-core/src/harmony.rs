use crate::{
    Chord, MidiState, MusicalColor, PITCH_CLASS_COUNT, chord::recognize_pitch_class_mask,
    compose_musical_color,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HarmonicState {
    pub pitch_class_weights: [f32; PITCH_CLASS_COUNT],
    pub active_pitch_classes: u16,
    pub active_note_count: usize,
    pub chord: Chord,
    pub color: MusicalColor,
    pub consonance: f32,
    pub tension: f32,
}

impl Default for HarmonicState {
    fn default() -> Self {
        Self {
            pitch_class_weights: [0.0; PITCH_CLASS_COUNT],
            active_pitch_classes: 0,
            active_note_count: 0,
            chord: Chord::Empty,
            color: MusicalColor::BLACK,
            consonance: 0.0,
            tension: 0.0,
        }
    }
}

impl HarmonicState {
    pub fn from_midi(state: &MidiState) -> Self {
        let pitch_class_weights = state.pitch_class_weights();
        let (active_pitch_classes, active_note_count) = state.active_summary();
        let chord = recognize_pitch_class_mask(active_pitch_classes, active_note_count);
        let color = compose_musical_color(pitch_class_weights);
        let consonance = consonance(&pitch_class_weights);
        let tension = if active_note_count == 0 {
            0.0
        } else {
            (1.0 - consonance).clamp(0.0, 1.0)
        };

        Self {
            pitch_class_weights,
            active_pitch_classes,
            active_note_count,
            chord,
            color,
            consonance,
            tension,
        }
    }

    pub fn is_finite(&self) -> bool {
        self.pitch_class_weights
            .iter()
            .all(|value| value.is_finite())
            && self.color.is_finite()
            && self.consonance.is_finite()
            && self.tension.is_finite()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SmoothedHarmonicState {
    current: HarmonicState,
}

impl SmoothedHarmonicState {
    pub const fn new(initial: HarmonicState) -> Self {
        Self { current: initial }
    }

    pub const fn current(&self) -> &HarmonicState {
        &self.current
    }

    pub fn advance(&mut self, target: &HarmonicState, delta_seconds: f32, response_seconds: f32) {
        let alpha = smoothing_alpha(delta_seconds, response_seconds);

        for (current, target) in self
            .current
            .pitch_class_weights
            .iter_mut()
            .zip(target.pitch_class_weights)
        {
            *current = lerp(*current, target, alpha);
        }

        self.current.color.red = lerp(self.current.color.red, target.color.red, alpha);
        self.current.color.green = lerp(self.current.color.green, target.color.green, alpha);
        self.current.color.blue = lerp(self.current.color.blue, target.color.blue, alpha);
        self.current.consonance = lerp(self.current.consonance, target.consonance, alpha);
        self.current.tension = lerp(self.current.tension, target.tension, alpha);
        self.current.active_pitch_classes = target.active_pitch_classes;
        self.current.active_note_count = target.active_note_count;
        self.current.chord = target.chord;
    }
}

pub fn smoothing_alpha(delta_seconds: f32, response_seconds: f32) -> f32 {
    if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
        return 0.0;
    }

    if !response_seconds.is_finite() || response_seconds <= 0.0 {
        return 1.0;
    }

    (delta_seconds / (response_seconds + delta_seconds)).clamp(0.0, 1.0)
}

fn lerp(from: f32, to: f32, alpha: f32) -> f32 {
    from + (to - from) * alpha
}

// Artistic/perceptual heuristic by interval class, not a physical acoustic model.
fn consonance(weights: &[f32; PITCH_CLASS_COUNT]) -> f32 {
    const INTERVAL_CONSONANCE: [f32; 7] = [1.0, 0.05, 0.35, 0.75, 0.85, 0.95, 0.1];

    let active_count = weights.iter().filter(|weight| **weight > 0.0).count();
    if active_count == 0 {
        return 0.0;
    }
    if active_count == 1 {
        return 1.0;
    }

    let mut weighted_score = 0.0;
    let mut pair_weight_total = 0.0;

    for left in 0..PITCH_CLASS_COUNT {
        for right in (left + 1)..PITCH_CLASS_COUNT {
            let pair_weight = weights[left] * weights[right];
            if pair_weight <= 0.0 {
                continue;
            }

            let distance = right - left;
            let interval_class = distance.min(PITCH_CLASS_COUNT - distance);
            weighted_score += pair_weight * INTERVAL_CONSONANCE[interval_class];
            pair_weight_total += pair_weight;
        }
    }

    if pair_weight_total <= f32::EPSILON {
        1.0
    } else {
        (weighted_score / pair_weight_total).clamp(0.0, 1.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::MidiEvent;

    use super::*;

    fn state_for(notes: &[u8]) -> HarmonicState {
        let mut midi = MidiState::default();
        for note in notes {
            midi.apply(MidiEvent::NoteOn {
                channel: 0,
                note: *note,
                velocity: 100,
            })
            .unwrap();
        }
        HarmonicState::from_midi(&midi)
    }

    #[test]
    fn empty_chord_has_zeroed_finite_state() {
        let state = HarmonicState::from_midi(&MidiState::default());
        assert_eq!(state.chord, Chord::Empty);
        assert_eq!(state.color, MusicalColor::BLACK);
        assert_eq!(state.consonance, 0.0);
        assert_eq!(state.tension, 0.0);
        assert!(state.is_finite());
    }

    #[test]
    fn consonance_and_tension_distinguish_intervals() {
        let perfect_fifth = state_for(&[60, 67]);
        let tritone = state_for(&[60, 66]);

        assert!(perfect_fifth.consonance > tritone.consonance);
        assert!(perfect_fifth.tension < tritone.tension);
        assert!((perfect_fifth.consonance + perfect_fifth.tension - 1.0).abs() < 1.0e-6);
    }

    #[test]
    fn analysis_never_produces_nan_or_infinity() {
        let state = state_for(&[0, 1, 12, 60, 64, 67, 127]);
        assert!(state.is_finite());
    }

    #[test]
    fn temporal_interpolation_is_bounded_and_deterministic() {
        let target = state_for(&[60]);
        let mut first = SmoothedHarmonicState::default();
        let mut second = SmoothedHarmonicState::default();

        first.advance(&target, 0.1, 0.1);
        second.advance(&target, 0.1, 0.1);

        assert_eq!(first, second);
        assert!((first.current().color.red - 0.5).abs() < 1.0e-6);
        assert!(first.current().is_finite());
    }
}
