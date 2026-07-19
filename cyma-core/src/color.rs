use crate::{PITCH_CLASS_COUNT, PitchClass, midi::normalize_weights};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MusicalColor {
    pub red: f32,
    pub green: f32,
    pub blue: f32,
}

impl MusicalColor {
    pub const BLACK: Self = Self {
        red: 0.0,
        green: 0.0,
        blue: 0.0,
    };

    pub fn is_finite(self) -> bool {
        self.red.is_finite() && self.green.is_finite() && self.blue.is_finite()
    }

    pub fn to_rgb8(self) -> (u8, u8, u8) {
        fn channel(value: f32) -> u8 {
            if value.is_finite() {
                (value.clamp(0.0, 1.0) * 255.0).round() as u8
            } else {
                0
            }
        }

        (channel(self.red), channel(self.green), channel(self.blue))
    }
}

// Artistic palette: pitch classes follow the circle of fifths around a hue wheel.
const PITCH_CLASS_COLORS: [MusicalColor; PITCH_CLASS_COUNT] = [
    MusicalColor {
        red: 1.0,
        green: 0.0,
        blue: 0.0,
    },
    MusicalColor {
        red: 0.0,
        green: 0.5,
        blue: 1.0,
    },
    MusicalColor {
        red: 1.0,
        green: 1.0,
        blue: 0.0,
    },
    MusicalColor {
        red: 0.5,
        green: 0.0,
        blue: 1.0,
    },
    MusicalColor {
        red: 0.0,
        green: 1.0,
        blue: 0.0,
    },
    MusicalColor {
        red: 1.0,
        green: 0.0,
        blue: 0.5,
    },
    MusicalColor {
        red: 0.0,
        green: 1.0,
        blue: 1.0,
    },
    MusicalColor {
        red: 1.0,
        green: 0.5,
        blue: 0.0,
    },
    MusicalColor {
        red: 0.0,
        green: 0.0,
        blue: 1.0,
    },
    MusicalColor {
        red: 0.5,
        green: 1.0,
        blue: 0.0,
    },
    MusicalColor {
        red: 1.0,
        green: 0.0,
        blue: 1.0,
    },
    MusicalColor {
        red: 0.0,
        green: 1.0,
        blue: 0.5,
    },
];

pub fn pitch_class_color(pitch_class: PitchClass) -> MusicalColor {
    PITCH_CLASS_COLORS[pitch_class.index()]
}

pub fn compose_musical_color(weights: [f32; PITCH_CLASS_COUNT]) -> MusicalColor {
    let weights = normalize_weights(weights);
    let mut color = MusicalColor::BLACK;

    for (weight, source) in weights.iter().zip(PITCH_CLASS_COLORS) {
        color.red += weight * source.red;
        color.green += weight * source.green;
        color.blue += weight * source.blue;
    }

    color
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_weighted_color() {
        let mut weights = [0.0; PITCH_CLASS_COUNT];
        weights[0] = 1.0;
        assert_eq!(
            compose_musical_color(weights),
            MusicalColor {
                red: 1.0,
                green: 0.0,
                blue: 0.0,
            }
        );

        weights[4] = 1.0;
        assert_eq!(
            compose_musical_color(weights),
            MusicalColor {
                red: 0.5,
                green: 0.5,
                blue: 0.0,
            }
        );
    }

    #[test]
    fn empty_color_is_black_and_finite() {
        let color = compose_musical_color([0.0; PITCH_CLASS_COUNT]);
        assert_eq!(color, MusicalColor::BLACK);
        assert!(color.is_finite());
    }

    #[test]
    fn pitch_class_palette_is_octave_independent() {
        assert_eq!(
            pitch_class_color(PitchClass::from_midi(60)),
            pitch_class_color(PitchClass::from_midi(72))
        );
        assert_eq!(pitch_class_color(PitchClass::C).to_rgb8(), (255, 0, 0));
    }

    #[test]
    fn pitch_class_palette_is_finite_and_deterministic() {
        for index in 0..PITCH_CLASS_COUNT as u8 {
            let pitch_class = PitchClass::from_index(index).unwrap();
            let first = pitch_class_color(pitch_class);
            let second = pitch_class_color(pitch_class);

            assert_eq!(first, second);
            assert!(first.is_finite());
            assert!(first.red >= 0.0 && first.red <= 1.0);
            assert!(first.green >= 0.0 && first.green <= 1.0);
            assert!(first.blue >= 0.0 && first.blue <= 1.0);
        }
    }

    #[test]
    fn rgb8_conversion_sanitizes_non_finite_channels() {
        let color = MusicalColor {
            red: f32::NAN,
            green: f32::INFINITY,
            blue: 0.5,
        };
        assert_eq!(color.to_rgb8(), (0, 0, 128));
    }
}
