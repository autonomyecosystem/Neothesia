#![cfg(target_os = "windows")]

use std::{hint::black_box, time::Instant};

use cyma_core::{HarmonicState, MidiEvent, MidiState, ModalField, SmoothedHarmonicState};

#[test]
#[ignore = "manual Windows release performance probe"]
fn harmonic_update_stays_below_one_millisecond() {
    assert!(
        is_release_build(),
        "run this probe with cargo test --release"
    );

    const WARMUP_ITERATIONS: usize = 1_000;
    const MEASURED_ITERATIONS: usize = 20_000;
    const HARMONIC_BUDGET_MS: f64 = 1.0;

    let mut midi = MidiState::default();
    for note in [36, 48, 55, 60, 64, 67, 72, 76, 79, 84] {
        midi.apply(MidiEvent::NoteOn {
            channel: 0,
            note,
            velocity: 100,
        })
        .unwrap();
    }

    let mut smoothed = SmoothedHarmonicState::default();
    for _ in 0..WARMUP_ITERATIONS {
        let harmonic = HarmonicState::from_midi(black_box(&midi));
        smoothed.advance(&harmonic, 1.0 / 60.0, 0.16);
        black_box(ModalField::from_harmonic(smoothed.current()));
    }

    let started = Instant::now();
    for _ in 0..MEASURED_ITERATIONS {
        let harmonic = HarmonicState::from_midi(black_box(&midi));
        smoothed.advance(&harmonic, 1.0 / 60.0, 0.16);
        black_box(ModalField::from_harmonic(smoothed.current()));
    }
    let average_ms = started.elapsed().as_secs_f64() * 1_000.0 / MEASURED_ITERATIONS as f64;
    eprintln!("Cyma harmonic update average: {average_ms:.6} ms/update");

    assert!(
        average_ms < HARMONIC_BUDGET_MS,
        "harmonic update average {average_ms:.6} ms exceeds {HARMONIC_BUDGET_MS:.1} ms"
    );
}

fn is_release_build() -> bool {
    !cfg!(debug_assertions)
}
