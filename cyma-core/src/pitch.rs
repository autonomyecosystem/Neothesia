pub const PITCH_CLASS_COUNT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PitchClass(u8);

impl PitchClass {
    pub const C: Self = Self(0);

    pub const fn from_midi(note: u8) -> Self {
        Self(note % PITCH_CLASS_COUNT as u8)
    }

    pub const fn from_index(index: u8) -> Option<Self> {
        if index < PITCH_CLASS_COUNT as u8 {
            Some(Self(index))
        } else {
            None
        }
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }

    pub const fn name(self) -> &'static str {
        const NAMES: [&str; PITCH_CLASS_COUNT] = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];

        NAMES[self.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_midi_note_to_pitch_class() {
        assert_eq!(PitchClass::from_midi(60), PitchClass::C);
        assert_eq!(PitchClass::from_midi(61).name(), "C#");
    }

    #[test]
    fn conversion_is_octave_independent() {
        assert_eq!(PitchClass::from_midi(48), PitchClass::from_midi(60));
        assert_eq!(PitchClass::from_midi(60), PitchClass::from_midi(72));
    }
}
