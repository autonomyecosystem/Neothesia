use std::sync::Arc;

use crate::{
    NeothesiaEvent, TransformUniform,
    config::Config,
    cyma::{CymaMidiSource, CymaState},
    input_manager::InputManager,
    output_manager::OutputManager,
    utils::window::WindowState,
};
use cyma_core::HarmonicState;
use midi_file::midly::MidiMessage;
use neothesia_core::render::{QuadRendererFactory, TextRendererFactory};
use wgpu_jumpstart::{Gpu, Uniform};
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
        let cyma = CymaState::new(config.cyma().enabled);

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

    pub fn update_cyma(&mut self, delta: std::time::Duration) {
        let response_seconds = self.config.cyma().response_time_seconds();
        self.cyma.update(delta, response_seconds);
    }

    pub fn cyma_harmonic_state(&self) -> Option<&HarmonicState> {
        self.cyma.harmonic_state()
    }
}
