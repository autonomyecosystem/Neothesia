use std::fmt::Write;

use cyma_core::{CymaQuality, CymaVisualization, HarmonicState, PITCH_CLASS_COUNT, PitchClass};

use crate::context::Context;

const FRAME_BUDGET_60_FPS_MS: f32 = 1_000.0 / 60.0;

pub(crate) fn build(ctx: &Context, ui: &mut nuon::Ui, top_offset: f32) {
    if !ctx.cyma_enabled() || !ctx.config.cyma().hud_enabled {
        return;
    }

    let Some(harmonic) = ctx.cyma_harmonic_state() else {
        return;
    };
    let model = CymaHudModel::new(
        harmonic,
        ctx.config.cyma().visualization,
        ctx.config.cyma().quality,
        ctx.cyma_cpu_ms(),
        ctx.cyma_gpu_ms(),
        ctx.cyma_gpu_timing_supported(),
        ctx.cyma_renderer_status(),
    );

    const CARD_WIDTH: f32 = 326.0;
    const CARD_HEIGHT: f32 = 214.0;
    const MARGIN: f32 = 12.0;
    let x = (ctx.window_state.logical_size.width - CARD_WIDTH - MARGIN).max(MARGIN);
    let y = top_offset.max(MARGIN);

    nuon::layer().overlay(true).build(ui, |ui| {
        nuon::translate().x(x).y(y).build(ui, |ui| {
            nuon::quad()
                .size(CARD_WIDTH, CARD_HEIGHT)
                .color([0.035, 0.031, 0.05, 0.92])
                .border_radius([9.0; 4])
                .build(ui);
            nuon::quad()
                .x(12.0)
                .y(12.0)
                .size(5.0, 42.0)
                .color(model.color)
                .border_radius([2.5; 4])
                .build(ui);

            label(
                ui,
                "CYMA · HARMONIC VIEW",
                [26.0, 12.0, 286.0, 14.0],
                12.0,
                true,
            );
            label(ui, &model.chord, [26.0, 29.0, 286.0, 25.0], 22.0, true);
            label(
                ui,
                &model.pitch_classes,
                [16.0, 61.0, 296.0, 17.0],
                12.5,
                false,
            );

            metric_bar(
                ui,
                "Consonance · perceptual heuristic",
                model.consonance,
                16.0,
                86.0,
                [0.25, 0.82, 0.58, 1.0],
            );
            metric_bar(
                ui,
                "Tension · inverse heuristic",
                model.tension,
                16.0,
                118.0,
                [0.95, 0.36, 0.48, 1.0],
            );

            nuon::quad()
                .x(16.0)
                .y(153.0)
                .size(20.0, 20.0)
                .color(model.color)
                .border_radius([4.0; 4])
                .build(ui);
            label(
                ui,
                &model.mode_line,
                [44.0, 151.0, 266.0, 14.0],
                11.5,
                false,
            );
            label(
                ui,
                "Color: artistic mix · Modes: ideal membrane basis",
                [44.0, 164.0, 266.0, 13.0],
                10.5,
                false,
            );
            label(
                ui,
                &model.performance_line,
                [16.0, 187.0, 214.0, 14.0],
                11.0,
                false,
            );
            label(
                ui,
                model.budget.label(),
                [224.0, 187.0, 86.0, 14.0],
                11.0,
                true,
            );
        });
    });
}

fn label(ui: &mut nuon::Ui, text: &str, rect: [f32; 4], font_size: f32, bold: bool) {
    nuon::label()
        .text(text)
        .pos(rect[0], rect[1])
        .size(rect[2], rect[3])
        .font_size(font_size)
        .bold(bold)
        .text_justify(nuon::TextJustify::Left)
        .build(ui);
}

fn metric_bar(ui: &mut nuon::Ui, title: &str, value: f32, x: f32, y: f32, color: [f32; 4]) {
    const WIDTH: f32 = 294.0;
    label(ui, title, [x, y, WIDTH, 13.0], 10.5, false);
    nuon::quad()
        .x(x)
        .y(y + 16.0)
        .size(WIDTH, 7.0)
        .color([1.0, 1.0, 1.0, 0.12])
        .border_radius([3.5; 4])
        .build(ui);
    nuon::quad()
        .x(x)
        .y(y + 16.0)
        .size(WIDTH * value.clamp(0.0, 1.0), 7.0)
        .color(color)
        .border_radius([3.5; 4])
        .build(ui);
}

#[derive(Debug, Clone, PartialEq)]
struct CymaHudModel {
    chord: String,
    pitch_classes: String,
    consonance: f32,
    tension: f32,
    color: [f32; 4],
    mode_line: String,
    performance_line: String,
    budget: BudgetState,
}

impl CymaHudModel {
    #[allow(clippy::too_many_arguments)]
    fn new(
        harmonic: &HarmonicState,
        visualization: CymaVisualization,
        quality: CymaQuality,
        cpu_ms: Option<f32>,
        gpu_ms: Option<f32>,
        gpu_timing_supported: bool,
        renderer_status: &str,
    ) -> Self {
        let chord = if harmonic.active_note_count == 0 {
            "No active chord".to_string()
        } else {
            harmonic.chord.to_string()
        };

        let mut pitch_classes = String::with_capacity(52);
        pitch_classes.push_str("Pitch classes: ");
        let mut first = true;
        for index in 0..PITCH_CLASS_COUNT {
            if harmonic.active_pitch_classes & (1_u16 << index) == 0 {
                continue;
            }
            let Some(pitch_class) = PitchClass::from_index(index as u8) else {
                continue;
            };
            if !first {
                pitch_classes.push_str(" · ");
            }
            pitch_classes.push_str(pitch_class.name());
            first = false;
        }
        if first {
            pitch_classes.push_str("none");
        }

        let mode_line = format!(
            "{} · {} · {}",
            visualization.label(),
            quality.label(),
            renderer_status
        );
        let mut performance_line = String::with_capacity(48);
        performance_line.push_str("CPU ");
        push_metric(&mut performance_line, cpu_ms);
        performance_line.push_str(" · GPU ");
        if gpu_timing_supported {
            push_metric(&mut performance_line, gpu_ms);
        } else {
            performance_line.push_str("N/A");
        }

        Self {
            chord,
            pitch_classes,
            consonance: finite_unit(harmonic.consonance),
            tension: finite_unit(harmonic.tension),
            color: [
                finite_unit(harmonic.color.red),
                finite_unit(harmonic.color.green),
                finite_unit(harmonic.color.blue),
                1.0,
            ],
            mode_line,
            performance_line,
            budget: BudgetState::from_metrics(cpu_ms, gpu_ms),
        }
    }
}

fn push_metric(target: &mut String, value: Option<f32>) {
    if let Some(value) = value.filter(|value| value.is_finite() && *value >= 0.0) {
        let _ = write!(target, "{value:.2} ms");
    } else {
        target.push_str("N/A");
    }
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BudgetState {
    Measuring,
    Within60Fps,
    OverBudget,
}

impl BudgetState {
    fn from_metrics(cpu_ms: Option<f32>, gpu_ms: Option<f32>) -> Self {
        let cpu_ms = valid_metric(cpu_ms);
        let gpu_ms = valid_metric(gpu_ms);
        let peak_ms = match (cpu_ms, gpu_ms) {
            (Some(cpu), Some(gpu)) => Some(cpu.max(gpu)),
            (Some(cpu), None) => Some(cpu),
            (None, Some(gpu)) => Some(gpu),
            (None, None) => None,
        };

        match peak_ms {
            None => Self::Measuring,
            Some(value) if value <= FRAME_BUDGET_60_FPS_MS => Self::Within60Fps,
            Some(_) => Self::OverBudget,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Measuring => "MEASURING",
            Self::Within60Fps => "60 FPS OK",
            Self::OverBudget => "OVER BUDGET",
        }
    }
}

fn valid_metric(value: Option<f32>) -> Option<f32> {
    value.filter(|value| value.is_finite() && *value >= 0.0)
}

#[cfg(test)]
mod tests {
    use cyma_core::{HarmonicState, MidiEvent, MidiState};

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
    fn educational_model_uses_shared_harmonic_state() {
        let model = CymaHudModel::new(
            &harmonic_for(&[60, 64, 67]),
            CymaVisualization::Surface3d,
            CymaQuality::Medium,
            Some(0.4),
            Some(0.8),
            true,
            "Active",
        );

        assert_eq!(model.chord, "CM");
        assert_eq!(model.pitch_classes, "Pitch classes: C · E · G");
        assert!(model.mode_line.contains("3D Surface"));
        assert_eq!(model.budget, BudgetState::Within60Fps);
    }

    #[test]
    fn empty_and_invalid_values_are_safe() {
        let harmonic = HarmonicState {
            consonance: f32::NAN,
            tension: f32::INFINITY,
            ..HarmonicState::default()
        };
        let model = CymaHudModel::new(
            &harmonic,
            CymaVisualization::Field2d,
            CymaQuality::Low,
            Some(f32::NAN),
            None,
            false,
            "Active",
        );

        assert_eq!(model.chord, "No active chord");
        assert_eq!(model.pitch_classes, "Pitch classes: none");
        assert_eq!(model.consonance, 0.0);
        assert_eq!(model.tension, 0.0);
        assert_eq!(model.budget, BudgetState::Measuring);
        assert!(model.performance_line.contains("GPU N/A"));
    }

    #[test]
    fn budget_uses_the_slower_cyma_path() {
        assert_eq!(
            BudgetState::from_metrics(Some(2.0), Some(18.0)),
            BudgetState::OverBudget
        );
        assert_eq!(
            BudgetState::from_metrics(Some(16.0), Some(8.0)),
            BudgetState::Within60Fps
        );
    }
}
