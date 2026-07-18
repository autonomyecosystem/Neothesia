use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use crate::{
    NeothesiaEvent, TransformUniform,
    config::Config,
    cyma::{CymaMidiSource, CymaState},
    input_manager::InputManager,
    output_manager::OutputManager,
    utils::window::WindowState,
};
use cyma_core::{CymaVisualization, HarmonicState};
use midi_file::midly::MidiMessage;
use neothesia_core::render::{
    CymaRenderOptions, CymaRenderer, QuadRendererFactory, TextRendererFactory,
};
use wgpu_jumpstart::{Gpu, Uniform, wgpu};
use winit::event_loop::EventLoopProxy;

use winit::window::Window;

pub struct Context {
    pub window: Arc<Window>,

    pub window_state: WindowState,
    pub gpu: Gpu,

    pub transform: Uniform<TransformUniform>,
    pub text_renderer_factory: TextRendererFactory,
    pub quad_renderer_factory: QuadRendererFactory,

    pub output_manager: OutputManager,
    pub input_manager: InputManager,
    pub config: Config,
    cyma: CymaState,
    pub(crate) cyma_renderer: CymaRendererState,
    cyma_cpu_metrics: CymaCpuMetrics,

    pub proxy: EventLoopProxy<NeothesiaEvent>,

    /// Last frame timestamp
    pub frame_timestamp: std::time::Instant,

    #[cfg(debug_assertions)]
    pub fps_ticker: neothesia_core::utils::fps_ticker::Fps,
}

impl Drop for Context {
    fn drop(&mut self) {
        self.config.save();
    }
}

impl Context {
    pub fn new(
        window: Arc<Window>,
        window_state: WindowState,
        proxy: EventLoopProxy<NeothesiaEvent>,
        gpu: Gpu,
    ) -> Self {
        let transform_uniform = Uniform::new(
            &gpu.device,
            TransformUniform::default(),
            wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
        );

        let config = Config::new();
        let cyma_enabled = config.cyma().enabled;
        let cyma = CymaState::new(cyma_enabled);
        let cyma_renderer = if cyma_enabled {
            CymaRendererState::initialize(&gpu)
        } else {
            CymaRendererState::Uninitialized
        };

        let text_renderer_factory = TextRendererFactory::new(&gpu);
        let quad_renderer_factory = QuadRendererFactory::new(&gpu, &transform_uniform);

        Self {
            window,

            window_state,
            gpu,
            transform: transform_uniform,
            text_renderer_factory,
            quad_renderer_factory,

            output_manager: Default::default(),
            input_manager: InputManager::new(proxy.clone()),
            config,
            cyma,
            cyma_renderer,
            cyma_cpu_metrics: CymaCpuMetrics::default(),
            proxy,
            frame_timestamp: std::time::Instant::now(),

            #[cfg(debug_assertions)]
            fps_ticker: Default::default(),
        }
    }

    pub fn resize(&mut self) {
        self.transform.data.update(
            self.window_state.physical_size.width as f32,
            self.window_state.physical_size.height as f32,
            self.window_state.scale_factor as f32,
        );
        self.transform.update(&self.gpu.queue);
    }

    pub fn cyma_enabled(&self) -> bool {
        self.cyma.is_enabled()
    }

    pub fn set_cyma_enabled(&mut self, enabled: bool) {
        self.config.set_cyma_enabled(enabled);
        self.cyma.set_enabled(enabled);

        if enabled && matches!(&self.cyma_renderer, CymaRendererState::Uninitialized) {
            self.cyma_renderer = CymaRendererState::initialize(&self.gpu);
        }
        if !enabled {
            self.cyma_cpu_metrics = CymaCpuMetrics::default();
        }
    }

    pub fn observe_cyma_midi_event(
        &mut self,
        source: CymaMidiSource,
        channel: u8,
        message: &MidiMessage,
    ) {
        if let Err(err) = self.cyma.observe_midi_event(source, channel, message) {
            log::error!("Cyma rejected a MIDI event: {err}");
        }
    }

    pub fn reset_cyma_source(&mut self, source: CymaMidiSource) {
        if let Err(err) = self.cyma.reset_source(source) {
            log::error!("Cyma failed to reset MIDI state: {err}");
        }
    }

    pub fn update_cyma(&mut self, delta: Duration) {
        if !self.cyma.is_enabled() {
            self.cyma_cpu_metrics.update_ms = 0.0;
            return;
        }

        let started = Instant::now();
        let response_seconds = self.config.cyma().response_time_seconds();
        self.cyma.update(delta, response_seconds);

        let config = *self.config.cyma();
        if let (Some(field), CymaRendererState::Ready(renderer)) =
            (self.cyma.modal_field().copied(), &mut self.cyma_renderer)
        {
            renderer.update(
                &field,
                CymaRenderOptions {
                    visualization: config.visualization,
                    quality: config.quality,
                    particles_enabled: config.particles_enabled,
                },
                delta,
                self.window_state.physical_size.width,
                self.window_state.physical_size.height,
            );
        }

        self.cyma_cpu_metrics.update_ms = elapsed_ms(started);
    }

    pub fn render_cyma(
        &mut self,
        surface_view: &wgpu::TextureView,
        clear_color: wgpu::Color,
        extent: wgpu::Extent3d,
    ) -> bool {
        if !self.cyma.is_enabled() {
            return false;
        }

        let started = Instant::now();
        let rendered = match &mut self.cyma_renderer {
            CymaRendererState::Ready(renderer) => {
                renderer.render(&mut self.gpu.encoder, surface_view, clear_color, extent)
            }
            CymaRendererState::Uninitialized | CymaRendererState::Unavailable => false,
        };
        let total_ms = self.cyma_cpu_metrics.update_ms + elapsed_ms(started);
        self.cyma_cpu_metrics.record(total_ms);
        rendered
    }

    pub fn after_gpu_submit(&mut self) {
        if let CymaRendererState::Ready(renderer) = &mut self.cyma_renderer {
            renderer.after_submit(&self.gpu.device);
        }
    }

    pub fn cyma_harmonic_state(&self) -> Option<&HarmonicState> {
        self.cyma.harmonic_state()
    }

    pub fn cyma_renderer_status(&self) -> &'static str {
        if !self.cyma.is_enabled() {
            return "Disabled";
        }

        match &self.cyma_renderer {
            CymaRendererState::Uninitialized => "Not initialized",
            CymaRendererState::Ready(renderer) => {
                let capabilities = renderer.capabilities();
                if self.config.cyma().visualization == CymaVisualization::Surface3d
                    && !capabilities.surface_3d
                {
                    "2D fallback (3D unavailable)"
                } else if self.config.cyma().particles_enabled && !capabilities.particles {
                    "Active (particles unavailable)"
                } else {
                    "Active"
                }
            }
            CymaRendererState::Unavailable => "GPU unavailable",
        }
    }

    pub fn cyma_cpu_ms(&self) -> Option<f32> {
        self.cyma.is_enabled().then_some(())?;
        self.cyma_cpu_metrics.ema_ms
    }

    pub fn cyma_gpu_ms(&self) -> Option<f32> {
        if !self.cyma.is_enabled() {
            return None;
        }
        let CymaRendererState::Ready(renderer) = &self.cyma_renderer else {
            return None;
        };
        renderer.last_gpu_ms()
    }

    pub fn cyma_gpu_timing_supported(&self) -> bool {
        let CymaRendererState::Ready(renderer) = &self.cyma_renderer else {
            return false;
        };
        renderer.capabilities().gpu_timing
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct CymaCpuMetrics {
    update_ms: f32,
    ema_ms: Option<f32>,
}

impl CymaCpuMetrics {
    fn record(&mut self, frame_ms: f32) {
        if !frame_ms.is_finite() || frame_ms < 0.0 {
            return;
        }

        const EMA_ALPHA: f32 = 0.12;
        self.ema_ms = Some(match self.ema_ms {
            Some(previous) => previous + (frame_ms - previous) * EMA_ALPHA,
            None => frame_ms,
        });
    }
}

fn elapsed_ms(started: Instant) -> f32 {
    started.elapsed().as_secs_f32() * 1_000.0
}

pub(crate) enum CymaRendererState {
    Uninitialized,
    Ready(Box<CymaRenderer>),
    Unavailable,
}

impl CymaRendererState {
    fn initialize(gpu: &Gpu) -> Self {
        match CymaRenderer::new(gpu) {
            Ok(renderer) => Self::Ready(Box::new(renderer)),
            Err(err) => {
                log::error!("Cyma renderer is unavailable: {err}");
                Self::Unavailable
            }
        }
    }
}
