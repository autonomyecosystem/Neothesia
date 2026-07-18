use crate::{PITCH_CLASS_COUNT, midi::normalize_weights};

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
}
