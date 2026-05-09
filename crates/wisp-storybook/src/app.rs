//! eframe `App` for the storybook.
//!
//! Layout: top bar (story picker) + 4/5 center canvas + 1/5 right sidebar.
//! wisp renders to a `RenderTexture` matching the canvas size; the texture is
//! registered with egui via `register_native_texture` and shown as an
//! `egui::Image` — zero-copy, GPU-side.

use std::collections::BTreeMap;
use std::time::Instant;

use wisp::application::{AppConfig, Application};
use wisp::render::Renderer;
use wisp::{Color, RenderTexture, Stage};

use crate::stories::all_stories;
use crate::story::Story;

const CANVAS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

pub struct StorybookApp {
    wisp: Application,
    renderer: Renderer,
    stories: Vec<Story>,
    current: usize,
    stage: Stage,
    rt: Option<CanvasRender>,
    started: Instant,
}

struct CanvasRender {
    texture: RenderTexture,
    egui_id: egui::TextureId,
    width: u32,
    height: u32,
}

impl StorybookApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Borrow eframe's wgpu device so wisp shares the same GPU context.
        let render_state = cc
            .wgpu_render_state
            .as_ref()
            .expect("storybook requires wgpu renderer");
        let device = render_state.device.clone();
        let queue = render_state.queue.clone();
        let adapter = render_state.adapter.clone();
        // We don't use the instance after construction, but Application owns one.
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let wisp = Application::from_wgpu(
            instance,
            adapter,
            device,
            queue,
            AppConfig {
                width: 1024,
                height: 768,
                ..Default::default()
            },
        );

        let renderer = Renderer::new(&wisp, CANVAS_FORMAT).expect("renderer");
        let stories = all_stories();
        let mut stage = Stage::new();
        if let Some(first) = stories.first() {
            (first.build)(&wisp, &mut stage);
        }

        Self {
            wisp,
            renderer,
            stories,
            current: 0,
            stage,
            rt: None,
            started: Instant::now(),
        }
    }

    fn select_story(&mut self, idx: usize) {
        if idx >= self.stories.len() || idx == self.current {
            return;
        }
        self.current = idx;
        self.stage = Stage::new();
        let story = &self.stories[idx];
        (story.build)(&self.wisp, &mut self.stage);
        self.started = Instant::now();
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "size clamped to [1, 8192] before cast"
    )]
    fn ensure_canvas(
        &mut self,
        ctx: &egui::Context,
        frame: &mut eframe::Frame,
        size: egui::Vec2,
    ) -> Option<egui::TextureId> {
        let render_state = frame.wgpu_render_state()?;

        let w = size.x.clamp(1.0, 8192.0).round() as u32;
        let h = size.y.clamp(1.0, 8192.0).round() as u32;
        let w = w.max(1);
        let h = h.max(1);

        let needs_realloc = match &self.rt {
            Some(c) => c.width != w || c.height != h,
            None => true,
        };

        if needs_realloc {
            // Drop old egui registration first so the texture id is freed.
            if let Some(old) = self.rt.take() {
                let mut renderer = render_state.renderer.write();
                renderer.free_texture(&old.egui_id);
            }
            let texture = RenderTexture::with_format(&self.wisp, w, h, CANVAS_FORMAT);
            let id = {
                let mut renderer = render_state.renderer.write();
                renderer.register_native_texture(
                    &render_state.device,
                    texture.view(),
                    wgpu::FilterMode::Linear,
                )
            };
            self.rt = Some(CanvasRender {
                texture,
                egui_id: id,
                width: w,
                height: h,
            });
        }

        // Tick + render each frame.
        let t = self.started.elapsed().as_secs_f32();
        let story = &self.stories[self.current];
        story.tick(&mut self.stage, t);

        let canvas = self.rt.as_ref()?;
        let _stats = self.renderer.render_stage(
            &self.wisp,
            canvas.texture.view(),
            Color::rgba_u8(18, 18, 22, 255),
            &self.stage,
        );

        ctx.request_repaint();
        Some(canvas.egui_id)
    }
}

impl eframe::App for StorybookApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Top bar — category-grouped story picker.
        egui::TopBottomPanel::top("nav").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("wisp · storybook");
                ui.separator();

                let by_category = group_by_category(&self.stories);
                for (cat, idxs) in &by_category {
                    ui.menu_button(*cat, |ui| {
                        for idx in idxs {
                            let story = &self.stories[*idx];
                            let label = format!("{} {}", story.milestone, story.title);
                            if ui.selectable_label(*idx == self.current, label).clicked() {
                                self.select_story(*idx);
                                ui.close_menu();
                            }
                        }
                    });
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let s = &self.stories[self.current];
                    ui.label(
                        egui::RichText::new(format!("{} · {}", s.milestone, s.title)).strong(),
                    );
                });
            });
        });

        // Right sidebar — 1/5 width — write-up.
        egui::SidePanel::right("writeup")
            .resizable(false)
            .exact_width(ctx.screen_rect().width() * 0.2)
            .show(ctx, |ui| {
                let story = &self.stories[self.current];
                ui.add_space(8.0);
                ui.heading(story.title);
                ui.label(
                    egui::RichText::new(format!("{} · {}", story.category, story.milestone))
                        .small()
                        .weak(),
                );
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(story.writeup);
                });
            });

        // Center 4/5 — live canvas.
        egui::CentralPanel::default().show(ctx, |ui| {
            let avail = ui.available_size();
            if let Some(id) = self.ensure_canvas(ctx, frame, avail) {
                ui.image((id, avail));
            } else {
                ui.label("(canvas unavailable — wgpu render state missing)");
            }
        });
    }
}

fn group_by_category(stories: &[Story]) -> BTreeMap<&'static str, Vec<usize>> {
    let mut out: BTreeMap<&'static str, Vec<usize>> = BTreeMap::new();
    for (i, s) in stories.iter().enumerate() {
        out.entry(s.category).or_default().push(i);
    }
    out
}
