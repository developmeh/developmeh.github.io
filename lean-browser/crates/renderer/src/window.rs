//! Window mode (feature `window`): a winit window presented through a
//! softbuffer surface. The surface's buffer is shared memory (wl_shm /
//! X11 SHM), so it counts as `Shared_Dirty`, not `Private_Dirty`; the
//! renderer only ever holds the strip buffer privately (plan §7).
//!
//! Interaction (M2 scope): scrolling with the wheel and keyboard
//! (arrows, Page Up/Down, Space, Home, End), resizing (re-layout only;
//! breakpoint re-cascade is M3), and a left click on a link prints its
//! target URL to stdout. `q` or Escape quits.

use std::num::NonZeroU32;
use std::sync::Arc;

use lean_alloc::{scope, Tag};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::window::{Window, WindowId};

use crate::document::Document;
use crate::fonts::FontSet;
use crate::paint::{paint_viewport, PaintParams, StripBuffer};
use crate::text::TextEngine;

/// Scroll step for arrow keys and one wheel line, in CSS px.
const LINE_STEP: f32 = 40.0;

struct App {
    doc: Document,
    fonts: FontSet,
    text: TextEngine,
    initial: (u32, u32),
    scroll: f32,
    cursor: (f64, f64),
    window: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    strip: StripBuffer,
    error: Option<String>,
}

/// Opens the window and runs the event loop until it closes.
pub fn run(
    doc: Document,
    fonts: FontSet,
    text: TextEngine,
    viewport: (u32, u32),
    scroll: f32,
) -> Result<(), String> {
    let _tag = scope(Tag::Window);
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = App {
        doc,
        fonts,
        text,
        initial: viewport,
        scroll,
        cursor: (0.0, 0.0),
        window: None,
        surface: None,
        strip: StripBuffer::new(viewport.0),
        error: None,
    };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    match app.error.take() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

impl App {
    fn fail(&mut self, event_loop: &ActiveEventLoop, msg: String) {
        self.error = Some(msg);
        event_loop.exit();
    }

    /// Viewport in CSS px and the scale factor, from the window.
    fn css_viewport(window: &Window) -> ((f32, f32), f32) {
        let size = window.inner_size();
        let scale = window.scale_factor() as f32;
        (
            (
                (size.width as f32 / scale).max(1.0),
                (size.height as f32 / scale).max(1.0),
            ),
            scale,
        )
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self.scroll.clamp(0.0, self.doc.max_scroll()).round();
    }

    fn scroll_by(&mut self, dy: f32) {
        self.scroll += dy;
        self.clamp_scroll();
        if let Some(w) = &self.window {
            w.request_redraw();
        }
    }

    /// Re-lays out `tops[]` if the viewport changed since the last paint.
    fn sync_viewport(&mut self, viewport: (f32, f32)) {
        if self.doc.viewport() != viewport {
            self.doc.relayout(viewport, &self.fonts, &mut self.text);
            self.clamp_scroll();
        }
    }

    fn redraw(&mut self) -> Result<(), String> {
        let Some(window) = self.window.clone() else {
            return Ok(());
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return Ok(());
        };
        let (viewport, scale) = Self::css_viewport(&window);
        self.sync_viewport(viewport);

        let Some(surface) = self.surface.as_mut() else {
            return Ok(());
        };
        surface.resize(w, h).map_err(|e| e.to_string())?;
        let mut buffer = surface.buffer_mut().map_err(|e| e.to_string())?;

        let params = PaintParams {
            scroll_y: self.scroll,
            viewport_w: size.width as f32 / scale,
            viewport_h: size.height as f32 / scale,
            dpr: scale,
            background: self.doc.background(),
        };
        let tree = self
            .doc
            .layout_viewport(self.scroll, &self.fonts, &mut self.text);
        let width = size.width as usize;
        let mut row = 0usize;
        let (doc, fonts, text, strip) = (&self.doc, &self.fonts, &mut self.text, &mut self.strip);
        paint_viewport(
            doc.page(),
            fonts,
            text,
            &tree,
            &params,
            strip,
            |bytes, bw, rows| {
                // Premultiplied RGBA8 -> 0RGB u32; the canvas is opaque.
                let bw = bw as usize;
                for (i, px) in bytes.chunks_exact(4).enumerate() {
                    let y = row + i / bw;
                    let x = i % bw;
                    if y < size.height as usize && x < width {
                        buffer[y * width + x] =
                            (u32::from(px[0]) << 16) | (u32::from(px[1]) << 8) | u32::from(px[2]);
                    }
                }
                row += rows as usize;
                Ok(())
            },
        )?;
        drop(tree);
        buffer.present().map_err(|e| e.to_string())
    }

    fn click(&mut self) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let (_, scale) = Self::css_viewport(&window);
        let x = self.cursor.0 as f32 / scale;
        let y = self.cursor.1 as f32 / scale + self.scroll;
        let tree = self
            .doc
            .layout_viewport(self.scroll, &self.fonts, &mut self.text);
        if let Some(href) = self.doc.link_at(&tree, x, y) {
            // Navigation itself (spawning the loader) is M3; report the target.
            println!("{href}");
        }
    }

    fn key(&mut self, event_loop: &ActiveEventLoop, key: &Key) {
        let page = self.doc.viewport().1 - LINE_STEP;
        match key {
            Key::Named(NamedKey::ArrowDown) => self.scroll_by(LINE_STEP),
            Key::Named(NamedKey::ArrowUp) => self.scroll_by(-LINE_STEP),
            Key::Named(NamedKey::PageDown) | Key::Named(NamedKey::Space) => self.scroll_by(page),
            Key::Named(NamedKey::PageUp) => self.scroll_by(-page),
            Key::Named(NamedKey::Home) => self.scroll_by(-self.scroll),
            Key::Named(NamedKey::End) => self.scroll_by(self.doc.max_scroll()),
            Key::Named(NamedKey::Escape) => event_loop.exit(),
            Key::Character(c) if c == "q" => event_loop.exit(),
            _ => {}
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let _tag = scope(Tag::Window);
        let title = {
            let page = self.doc.page();
            if page.title.is_empty() {
                page.final_url.to_string()
            } else {
                page.title.to_string()
            }
        };
        let attrs = Window::default_attributes()
            .with_title(format!("{title} - Lean Browser"))
            .with_inner_size(LogicalSize::new(
                f64::from(self.initial.0),
                f64::from(self.initial.1),
            ));
        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => return self.fail(event_loop, format!("cannot create window: {e}")),
        };
        let context = match softbuffer::Context::new(window.clone()) {
            Ok(c) => c,
            Err(e) => return self.fail(event_loop, format!("softbuffer context: {e}")),
        };
        let surface = match softbuffer::Surface::new(&context, window.clone()) {
            Ok(s) => s,
            Err(e) => return self.fail(event_loop, format!("softbuffer surface: {e}")),
        };
        self.surface = Some(surface);
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Err(e) = self.redraw() {
                    self.fail(event_loop, e);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => -y * LINE_STEP,
                    MouseScrollDelta::PixelDelta(p) => {
                        let scale = self
                            .window
                            .as_ref()
                            .map_or(1.0, |w| w.scale_factor() as f32);
                        -(p.y as f32) / scale
                    }
                };
                self.scroll_by(dy);
            }
            WindowEvent::CursorMoved { position, .. } => self.cursor = (position.x, position.y),
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => self.click(),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => self.key(event_loop, &logical_key),
            _ => {}
        }
    }
}
