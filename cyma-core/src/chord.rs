use std::fmt;

use crate::{PITCH_CLASS_COUNT, PitchClass};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chord {
    Empty,
    Single(PitchClass),
    Named {
        root: PitchClass,
        quality: ChordQuality,
    },
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordQuality {
    Minor,
    Major,
    Diminished,
    Augmented,
    MinorSeventh,
    MajorSeventh,
    DominantSeventh,
    DiminishedSeventh,
    HalfDiminished,
    AugmentedSeventh,
    AugmentedMajorSeventh,
    MinorMajorSeventh,
    MajorSixth,
    MinorSixth,
    SuspendedSecond,
    SuspendedFourth,
    DominantSeventhSuspendedFourth,
    DominantNinth,
    MajorNinth,
    MinorNinth,
    Power,
}

impl ChordQuality {
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Minor => "m",
            Self::Major => "M",
            Self::Diminished => "dim",
            Self::Augmented => "aug",
            Self::MinorSeventh => "m7",
            Self::MajorSeventh => "maj7",
            Self::DominantSeventh => "7",
            Self::DiminishedSeventh => "dim7",
            Self::HalfDiminished => "m7b5",
            Self::AugmentedSeventh => "aug7",
            Self::AugmentedMajorSeventh => "augmaj7",
            Self::MinorMajorSeventh => "mmaj7",
            Self::MajorSixth => "6",
            Self::MinorSixth => "m6",
            Self::SuspendedSecond => "sus2",
            Self::SuspendedFourth => "sus4",
            Self::DominantSeventhSuspendedFourth => "7sus4",
            Self::DominantNinth => "9",
            Self::MajorNinth => "maj9",
            Self::MinorNinth => "m9",
            Self::Power => "5",
        }
    }
}

impl fmt::Display for Chord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("No notes"),
            Self::Single(note) => f.write_str(note.name()),
            Self::Named { root, quality } => write!(f, "{}{}", root.name(), quality.suffix()),
            Self::Unknown => Ok(()),
        }
    }
}

pub fn deduce_chord_name(midi_notes: &[u8]) -> String {
    recognize_midi_notes(midi_notes).to_string()
}

pub fn recognize_midi_notes(midi_notes: &[u8]) -> Chord {
    if midi_notes.is_empty() {
        return Chord::Empty;
    }

    if midi_notes.len() == 1 {
        return Chord::Single(PitchClass::from_midi(midi_notes[0]));
    }

    let mut pitch_class_mask = 0_u16;
    for note in midi_notes {
        pitch_class_mask |= 1 << PitchClass::from_midi(*note).index();
    }

    recognize_pitch_class_mask(pitch_class_mask, midi_notes.len())
}

pub(crate) fn recognize_pitch_class_mask(pitch_class_mask: u16, note_count: usize) -> Chord {
    if note_count == 0 || pitch_class_mask == 0 {
        return Chord::Empty;
    }

    if note_count == 1 {
        let root = pitch_class_mask.trailing_zeros() as u8;
        return PitchClass::from_index(root).map_or(Chord::Unknown, Chord::Single);
    }

    for root in 0..PITCH_CLASS_COUNT as u8 {
        if pitch_class_mask & (1 << root) == 0 {
            continue;
        }

        let intervals = interval_mask(pitch_class_mask, root);
        if let Some(quality) = match_chord_quality(intervals) {
            let root = PitchClass::from_index(root).unwrap_or(PitchClass::C);
            return Chord::Named { root, quality };
        }
    }

    Chord::Unknown
}

fn interval_mask(pitch_class_mask: u16, root: u8) -> u16 {
    let mut intervals = 0_u16;

    for note in 0..PITCH_CLASS_COUNT as u8 {
        if note != root && pitch_class_mask & (1 << note) != 0 {
            let interval = (note + PITCH_CLASS_COUNT as u8 - root) % PITCH_CLASS_COUNT as u8;
            intervals |= 1 << interval;
        }
    }

    intervals
}

const fn intervals(values: &[u8]) -> u16 {
    let mut mask = 0_u16;
    let mut index = 0;
    while index < values.len() {
        mask |= 1 << values[index];
        index += 1;
    }
    mask
}

fn match_chord_quality(intervals_mask: u16) -> Option<ChordQuality> {
    match intervals_mask {
        value if value == intervals(&[3, 7]) => Some(ChordQuality::Minor),
        value if value == intervals(&[4, 7]) => Some(ChordQuality::Major),
        value if value == intervals(&[3, 6]) => Some(ChordQuality::Diminished),
        value if value == intervals(&[4, 8]) => Some(ChordQuality::Augmented),
        value if value == intervals(&[3, 7, 10]) => Some(ChordQuality::MinorSeventh),
        value if value == intervals(&[4, 7, 11]) => Some(ChordQuality::MajorSeventh),
        value if value == intervals(&[4, 7, 10]) => Some(ChordQuality::DominantSeventh),
        value if value == intervals(&[3, 6, 9]) => Some(ChordQuality::DiminishedSeventh),
        value if value == intervals(&[3, 6, 10]) => Some(ChordQuality::HalfDiminished),
        value if value == intervals(&[4, 8, 10]) => Some(ChordQuality::AugmentedSeventh),
        value if value == intervals(&[4, 8, 11]) => Some(ChordQuality::AugmentedMajorSeventh),
        value if value == intervals(&[3, 7, 11]) => Some(ChordQuality::MinorMajorSeventh),
        value if value == intervals(&[4, 7, 9]) => Some(ChordQuality::MajorSixth),
        value if value == intervals(&[3, 7, 9]) => Some(ChordQuality::MinorSixth),
        value if value == intervals(&[2, 7]) => Some(ChordQuality::SuspendedSecond),
        value if value == intervals(&[5, 7]) => Some(ChordQuality::SuspendedFourth),
        value if value == intervals(&[5, 7, 10]) => {
            Some(ChordQuality::DominantSeventhSuspendedFourth)
        }
        value if value == intervals(&[2, 4, 7, 10]) => Some(ChordQuality::DominantNinth),
        value if value == intervals(&[2, 4, 7, 11]) => Some(ChordQuality::MajorNinth),
        value if value == intervals(&[2, 3, 7, 10]) => Some(ChordQuality::MinorNinth),
        value if value == intervals(&[7]) => Some(ChordQuality::Power),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_existing_freeplay_chords() {
        assert_eq!(deduce_chord_name(&[60, 64, 67]), "CM");
        assert_eq!(deduce_chord_name(&[62, 66, 69]), "DM");
        assert_eq!(deduce_chord_name(&[60, 63, 67]), "Cm");
        assert_eq!(deduce_chord_name(&[57, 60, 64]), "Am");
        assert_eq!(deduce_chord_name(&[60, 64, 67, 71]), "Cmaj7");
        assert_eq!(deduce_chord_name(&[60, 64, 67, 70]), "C7");
        assert_eq!(deduce_chord_name(&[60, 63, 67, 70]), "Cm7");
        assert_eq!(deduce_chord_name(&[60, 63, 66]), "Cdim");
        assert_eq!(deduce_chord_name(&[60, 64, 68]), "Caug");
        assert_eq!(deduce_chord_name(&[60, 65, 67]), "Csus4");
        assert_eq!(deduce_chord_name(&[60, 67]), "C5");
    }

    #[test]
    fn handles_empty_and_single_note_chords() {
        assert_eq!(deduce_chord_name(&[]), "No notes");
        assert_eq!(deduce_chord_name(&[60]), "C");
    }

    #[test]
    fn chord_recognition_is_octave_independent() {
        assert_eq!(deduce_chord_name(&[48, 64, 67, 72]), "CM");
        assert_eq!(
            deduce_chord_name(&[60, 64, 67]),
            deduce_chord_name(&[72, 76, 79])
        );
    }
}
