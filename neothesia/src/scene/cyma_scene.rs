use std::time::Duration;

use midi_file::midly::MidiMessage;
use neothesia_core::render::{GlowRenderer, QuadRenderer, TextRenderer};
use winit::{
    event::WindowEvent,
    keyboard::{Key, NamedKey},
};

use crate::{
    NeothesiaEvent,
    context::Context,
    cyma_hud, icons,
    scene::{MouseToMidiEventState, NuonRenderer, Scene, playing_scene::Keyboard},
    song::Song,
    utils::window::WinitEvent,
};

pub struct CymaScene {
    keyboard: Keyboard,
    text_renderer: TextRenderer,
    quad_renderer: QuadRenderer,
    glow: Option<GlowRenderer>,
    song: Option<Song>,
    nuon_renderer: NuonRenderer,
    nuon: nuon::Ui,
    mouse_to_midi_state: MouseToMidiEventState,
}

impl CymaScene {
    pub fn new(ctx: &mut Context, song: Option<Song>) -> Self {
        let mut keyboard = Keyboard::new(ctx, Default::default());
        keyboard.set_pressed_by_user_colors(ctx.config.color_schema()[0].clone());
        let glow = ctx.config.glow().then_some(GlowRenderer::new(
            &ctx.gpu,
            &ctx.transform,
            keyboard.layout(),
        ));

        Self {
            keyboard,
            text_renderer: ctx.text_renderer_factory.new_renderer(),
            quad_renderer: ctx.quad_renderer_factory.new_renderer(),
            glow,
            song,
            nuon_renderer: NuonRenderer::new(ctx),
            nuon: nuon::Ui::new(),
            mouse_to_midi_state: MouseToMidiEventState::default(),
        }
    }

    fn resize(&mut self, ctx: &Context) {
        self.keyboard.resize(ctx);
    }

    fn update_glow(&mut self, delta: Duration) {
        let Some(glow) = &mut self.glow else {
            return;
        };

        glow.clear();
        for (key, state) in self
            .keyboard
            .layout()
            .keys
            .iter()
            .zip(self.keyboard.key_states())
        {
            let Some(color) = state.pressed_by_user().copied() else {
                continue;
            };
            glow.push(
                key.id(),
                color,
                key.x(),
                self.keyboard.pos().y,
                key.width(),
                delta,
            );
        }
    }

    fn return_to_menu(&self, ctx: &Context) {
        ctx.proxy
            .send_event(NeothesiaEvent::MainMenu(self.song.clone()))
            .ok();
    }

    fn update_ui(&mut self, ctx: &Context) {
        let width = ctx.window_state.logical_size.width;
        let mut return_to_menu = false;
        nuon::layer().overlay(true).build(&mut self.nuon, |ui| {
            nuon::quad()
                .size(width, 42.0)
                .color([0.055, 0.048, 0.072, 0.94])
                .build(ui);

            if nuon::button()
                .pos(6.0, 6.0)
                .size(30.0, 30.0)
                .border_radius([5.0; 4])
                .icon(icons::left_arrow_icon())
                .build(ui)
            {
                return_to_menu = true;
            }

            nuon::label()
                .text("CYMA · 1 m² CHLADNI PLATE")
                .pos(46.0, 9.0)
                .size((width - 58.0).max(0.0), 24.0)
                .font_size(16.0)
                .bold(true)
                .text_justify(nuon::TextJustify::Left)
                .build(ui);
        });
        if return_to_menu {
            self.return_to_menu(ctx);
        }
        cyma_hud::build(ctx, &mut self.nuon, 54.0);
    }
}

impl Scene for CymaScene {
    fn update(&mut self, ctx: &mut Context, delta: Duration) {
        self.quad_renderer.clear();
        self.keyboard
            .update(&mut self.quad_renderer, &mut self.text_renderer);
        self.update_glow(delta);
        self.update_ui(ctx);
        super::render_nuon(&mut self.nuon, &mut self.nuon_renderer, ctx);

        self.quad_renderer.prepare();
        if let Some(glow) = &mut self.glow {
            glow.prepare();
        }
        self.text_renderer.update(
            ctx.window_state.physical_size,
            ctx.window_state.scale_factor as f32,
        );
    }

    fn render<'pass>(&'pass mut self, rpass: &mut wgpu_jumpstart::RenderPass<'pass>) {
        self.quad_renderer.render(rpass);
        if let Some(glow) = &self.glow {
            glow.render(rpass);
        }
        self.text_renderer.render(rpass);
        self.nuon_renderer.render(rpass);
    }

    fn window_event(&mut self, ctx: &mut Context, event: &WindowEvent) {
        if event.window_resized() || event.scale_factor_changed() {
            self.resize(ctx);
        }

        if event.back_mouse_pressed() || event.key_released(Key::Named(NamedKey::Escape)) {
            self.return_to_menu(ctx);
        }

        super::handle_nuon_window_event(&mut self.nuon, event, ctx);
        super::handle_pc_keyboard_to_midi_event(ctx, event);
        super::handle_mouse_to_midi_event(
            &mut self.keyboard,
            &mut self.mouse_to_midi_state,
            ctx,
            event,
        );
    }

    fn midi_event(&mut self, ctx: &mut Context, _channel: u8, message: &MidiMessage) {
        self.keyboard.user_midi_event(&ctx.config, message);
        ctx.output_manager
            .connection()
            .midi_event(0.into(), *message);
    }
}
