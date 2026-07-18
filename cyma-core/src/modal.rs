use crate::{HarmonicState, MusicalColor, PITCH_CLASS_COUNT};

pub const MAX_MODAL_COMPONENTS: usize = 12;

const MIN_COMPONENT_AMPLITUDE: f32 = 1.0e-4;
const PHASE_STEP: f32 = std::f32::consts::TAU / PITCH_CLASS_COUNT as f32;

// Artistic pitch-class mapping to standing-wave modes of an ideal rectangular membrane.
// The modal basis is physically motivated; the association with pitch classes is not.
const PITCH_CLASS_MODES: [(u8, u8); PITCH_CLASS_COUNT] = [
    (1, 1),
    (1, 2),
    (2, 1),
    (2, 2),
    (1, 3),
    (3, 1),
    (2, 3),
    (3, 2),
    (1, 4),
    (4, 1),
    (3, 3),
    (2, 4),
];

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ModalComponent {
    pub mode_x: u8,
    pub mode_y: u8,
    pub amplitude: f32,
    pub phase: f32,
}

impl ModalComponent {
    pub fn is_finite(self) -> bool {
        self.amplitude.is_finite() && self.phase.is_finite()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModalField {
    components: [ModalComponent; MAX_MODAL_COMPONENTS],
    component_count: u8,
    pub color: MusicalColor,
    pub activity: f32,
    pub consonance: f32,
    pub tension: f32,
}

impl Default for ModalField {
    fn default() -> Self {
        Self {
            components: [ModalComponent::default(); MAX_MODAL_COMPONENTS],
            component_count: 0,
            color: MusicalColor::BLACK,
            activity: 0.0,
            consonance: 0.0,
            tension: 0.0,
        }
    }
}

impl ModalField {
    pub fn from_harmonic(harmonic: &HarmonicState) -> Self {
        let mut field = Self {
            color: MusicalColor {
                red: finite_unit(harmonic.color.red),
                green: finite_unit(harmonic.color.green),
                blue: finite_unit(harmonic.color.blue),
            },
            consonance: finite_unit(harmonic.consonance),
            tension: finite_unit(harmonic.tension),
            ..Self::default()
        };

        for (pitch_class, weight) in harmonic.pitch_class_weights.iter().copied().enumerate() {
            let amplitude = finite_unit(weight);
            if amplitude < MIN_COMPONENT_AMPLITUDE {
                continue;
            }

            let component_index = usize::from(field.component_count);
            if component_index >= MAX_MODAL_COMPONENTS {
                break;
            }

            let (mode_x, mode_y) = PITCH_CLASS_MODES[pitch_class];
            field.components[component_index] = ModalComponent {
                mode_x,
                mode_y,
                amplitude,
                phase: pitch_class as f32 * PHASE_STEP,
            };
            field.component_count += 1;
            field.activity += amplitude;
        }

        field.activity = field.activity.clamp(0.0, 1.0);
        field
    }

    pub fn components(&self) -> &[ModalComponent] {
        &self.components[..usize::from(self.component_count)]
    }

    pub const fn component_count(&self) -> u8 {
        self.component_count
    }

    pub fn is_active(&self) -> bool {
        self.component_count > 0 && self.activity > 0.0
    }

    pub fn is_finite(&self) -> bool {
        self.color.is_finite()
            && self.activity.is_finite()
            && self.consonance.is_finite()
            && self.tension.is_finite()
            && self
                .components()
                .iter()
                .all(|component| component.is_finite())
    }
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use crate::{MidiEvent, MidiState};

    use super::*;

    fn harmonic_for(notes: &[u8]) -> HarmonicState {
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
    fn empty_harmony_produces_an_empty_finite_field() {
        let field = ModalField::from_harmonic(&HarmonicState::default());
        assert_eq!(field.component_count(), 0);
        assert!(!field.is_active());
        assert!(field.is_finite());
    }

    #[test]
    fn maps_pitch_classes_deterministically() {
        let harmonic = harmonic_for(&[60, 64, 67]);
        let first = ModalField::from_harmonic(&harmonic);
        let second = ModalField::from_harmonic(&harmonic);

        assert_eq!(first, second);
        assert_eq!(first.component_count(), 3);
        assert_eq!(
            (first.components()[0].mode_x, first.components()[0].mode_y),
            (1, 1)
        );
        assert_eq!(
            (first.components()[1].mode_x, first.components()[1].mode_y),
            (1, 3)
        );
        assert_eq!(
            (first.components()[2].mode_x, first.components()[2].mode_y),
            (3, 2)
        );
    }

    #[test]
    fn supports_at_most_twelve_normalized_components() {
        let harmonic = harmonic_for(&(60_u8..72).collect::<Vec<_>>());
        let field = ModalField::from_harmonic(&harmonic);
        let amplitude_sum = field
            .components()
            .iter()
            .map(|component| component.amplitude)
            .sum::<f32>();

        assert_eq!(usize::from(field.component_count()), MAX_MODAL_COMPONENTS);
        assert!((amplitude_sum - 1.0).abs() < 1.0e-6);
        assert!((field.activity - 1.0).abs() < 1.0e-6);
        assert!(field.is_finite());
    }

    #[test]
    fn sanitizes_non_finite_harmonic_values() {
        let harmonic = HarmonicState {
            pitch_class_weights: [f32::NAN; PITCH_CLASS_COUNT],
            color: MusicalColor {
                red: f32::INFINITY,
                green: f32::NEG_INFINITY,
                blue: f32::NAN,
            },
            consonance: f32::NAN,
            tension: f32::INFINITY,
            ..HarmonicState::default()
        };

        let field = ModalField::from_harmonic(&harmonic);
        assert_eq!(field, ModalField::default());
        assert!(field.is_finite());
    }
}
