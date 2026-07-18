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
    pub phase_gain: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ModalSample {
    pub value: f32,
    pub node_gradient: [f32; 2],
}

impl ModalSample {
    pub fn is_finite(self) -> bool {
        self.value.is_finite()
            && self
                .node_gradient
                .iter()
                .all(|component| component.is_finite())
    }
}

impl ModalComponent {
    pub fn is_finite(self) -> bool {
        self.amplitude.is_finite() && self.phase_gain.is_finite()
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
            let phase = pitch_class as f32 * PHASE_STEP;
            let modal_frequency = f32::from(mode_x * mode_x + mode_y * mode_y).sqrt();
            field.components[component_index] = ModalComponent {
                mode_x,
                mode_y,
                amplitude,
                phase_gain: 0.75 + 0.25 * (modal_frequency * 0.25 + phase).cos(),
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

    pub fn sample(&self, x: f32, y: f32) -> f32 {
        self.sample_with_node_gradient(x, y).value
    }

    pub fn sample_with_node_gradient(&self, x: f32, y: f32) -> ModalSample {
        if !x.is_finite() || !y.is_finite() || self.activity <= f32::EPSILON {
            return ModalSample::default();
        }

        let x = x.clamp(0.0, 1.0);
        let y = y.clamp(0.0, 1.0);
        let mut value = 0.0;
        let mut gradient = [0.0; 2];

        for component in self.components() {
            let frequency_x = std::f32::consts::PI * f32::from(component.mode_x);
            let frequency_y = std::f32::consts::PI * f32::from(component.mode_y);
            let phase_x = frequency_x * x;
            let phase_y = frequency_y * y;
            let sin_x = phase_x.sin();
            let sin_y = phase_y.sin();
            let gain = component.amplitude * component.phase_gain;

            value += gain * sin_x * sin_y;
            gradient[0] += gain * frequency_x * phase_x.cos() * sin_y;
            gradient[1] += gain * frequency_y * sin_x * phase_y.cos();
        }

        let normalization = self.activity.max(MIN_COMPONENT_AMPLITUDE);
        value /= normalization;
        gradient[0] /= normalization;
        gradient[1] /= normalization;

        let direction = if value > 0.0 {
            1.0
        } else if value < 0.0 {
            -1.0
        } else {
            0.0
        };
        let sample = ModalSample {
            value,
            node_gradient: [gradient[0] * direction, gradient[1] * direction],
        };

        if sample.is_finite() {
            sample
        } else {
            ModalSample::default()
        }
    }

    pub fn node_gradient(&self, x: f32, y: f32) -> [f32; 2] {
        self.sample_with_node_gradient(x, y).node_gradient
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

    #[test]
    fn samples_nodes_and_gradients_deterministically() {
        let field = ModalField::from_harmonic(&harmonic_for(&[60, 64, 67]));
        assert!(field.sample(0.0, 0.5).abs() < 1.0e-6);
        assert!(field.sample(0.5, 0.0).abs() < 1.0e-6);

        let first = field.node_gradient(0.37, 0.61);
        let second = field.node_gradient(0.37, 0.61);
        assert_eq!(first, second);
        assert!(first.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn analytical_node_gradient_matches_finite_difference() {
        let field = ModalField::from_harmonic(&harmonic_for(&[60, 64, 67]));
        let x = 0.37;
        let y = 0.61;
        let epsilon = 1.0e-3;
        let expected = [
            (field.sample(x + epsilon, y).abs() - field.sample(x - epsilon, y).abs())
                / (2.0 * epsilon),
            (field.sample(x, y + epsilon).abs() - field.sample(x, y - epsilon).abs())
                / (2.0 * epsilon),
        ];
        let analytical = field.node_gradient(x, y);

        assert!((analytical[0] - expected[0]).abs() < 2.0e-3);
        assert!((analytical[1] - expected[1]).abs() < 2.0e-3);
    }

    #[test]
    fn invalid_sample_inputs_are_zeroed() {
        let field = ModalField::from_harmonic(&harmonic_for(&[60]));
        assert_eq!(field.sample(f32::NAN, 0.5), 0.0);
        assert_eq!(field.node_gradient(0.5, f32::NAN), [0.0; 2]);
        assert_eq!(
            field.sample_with_node_gradient(f32::INFINITY, 0.5),
            ModalSample::default()
        );
    }
}
