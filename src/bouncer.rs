use crate::{assets::Atlas, platform, renderer::LayeredRenderer};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tray_icon::{
    menu::{MenuEvent, MenuId},
    TrayIcon,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow},
    window::WindowId,
};

const UPDATE_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);
const BASE_SPEED_PIXELS_PER_SECOND: f64 = 1000.0;
const MAX_UPDATE_INTERVAL: Duration = Duration::from_millis(100);

pub struct Bouncer {
    atlas: Atlas,
    areas: Vec<(i32, i32, u32, u32)>,
    bounce_speed: f64,
    window: Option<platform::AppWindow>,
    renderer: Option<LayeredRenderer>,
    frame_index: usize,
    next_frame: Instant,
    last_update: Instant,
    position: (f64, f64),
    last_window_position: Option<(i32, i32)>,
    velocity: (f64, f64),
    _tray_icon: TrayIcon,
    quit_item_id: MenuId,
}

impl Bouncer {
    pub fn new(
        atlas: Atlas,
        areas: Vec<(i32, i32, u32, u32)>,
        bounce_speed: f64,
        tray_icon: TrayIcon,
        quit_item_id: MenuId,
    ) -> Self {
        let now = Instant::now();
        let initial_delay = atlas.delays[0];
        let position = Self::random_position(&areas, atlas.width, atlas.height);
        Self {
            atlas,
            areas,
            bounce_speed,
            window: None,
            renderer: None,
            frame_index: 0,
            next_frame: now + Duration::from_millis(initial_delay as u64),
            last_update: now,
            position,
            last_window_position: None,
            velocity: (0.32, 0.27),
            _tray_icon: tray_icon,
            quit_item_id,
        }
    }

    fn redraw(&mut self) {
        let Some(frame) = self.atlas.frame(self.frame_index) else {
            return;
        };
        if let Some(renderer) = self.renderer.as_mut() {
            renderer.present(frame);
        }
    }

    fn advance(&mut self, event_loop: &ActiveEventLoop) {
        if platform::native_close_requested() {
            event_loop.exit();
            return;
        }
        if let Ok(event) = MenuEvent::receiver().try_recv() {
            if event.id == self.quit_item_id {
                event_loop.exit();
                return;
            }
        }
        let now = Instant::now();
        let elapsed = now
            .saturating_duration_since(self.last_update)
            .min(MAX_UPDATE_INTERVAL)
            .as_secs_f64();
        self.last_update = now;
        let distance = BASE_SPEED_PIXELS_PER_SECOND * self.bounce_speed * elapsed;
        let previous_position = self.position;
        self.position.0 += self.velocity.0 * distance;
        self.position.1 += self.velocity.1 * distance;
        self.bounce_at_edges(previous_position);
        let frame_changed = if now >= self.next_frame {
            self.frame_index = (self.frame_index + 1) % self.atlas.delays.len();
            self.next_frame = now + Duration::from_millis(self.atlas.delays[self.frame_index] as u64);
            true
        } else {
            false
        };
        let window_position = (self.position.0.round() as i32, self.position.1.round() as i32);
        if self.last_window_position != Some(window_position) {
            if let Some(window) = self.window.as_ref() {
                platform::move_native_window(window, window_position.0, window_position.1);
                self.last_window_position = Some(window_position);
            }
        }
        if frame_changed {
            self.redraw();
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(now + UPDATE_INTERVAL));
    }

    fn bounce_at_edges(&mut self, previous: (f64, f64)) {
        if self.position_is_valid() {
            return;
        }
        let candidate = self.position;
        self.position.0 = candidate.0;
        self.position.1 = previous.1;
        if self.position_is_valid() {
            self.velocity.1 *= -1.0;
            return;
        }
        self.position.0 = previous.0;
        self.position.1 = candidate.1;
        if self.position_is_valid() {
            self.velocity.0 *= -1.0;
            return;
        }
        self.position = previous;
        self.velocity.0 *= -1.0;
        self.velocity.1 *= -1.0;
    }

    fn position_is_valid(&self) -> bool {
        let right = self.position.0 + f64::from(self.atlas.width) - f64::EPSILON;
        let bottom = self.position.1 + f64::from(self.atlas.height) - f64::EPSILON;
        [
            (self.position.0, self.position.1),
            (right, self.position.1),
            (self.position.0, bottom),
            (right, bottom),
        ]
            .iter()
            .all(|&point| self.areas.iter().any(|&(x, y, width, height)| {
                point.0 >= f64::from(x)
                    && point.0 < f64::from(x) + f64::from(width)
                    && point.1 >= f64::from(y)
                    && point.1 < f64::from(y) + f64::from(height)
            }))
    }

    fn random_position(areas: &[(i32, i32, u32, u32)], width: u32, height: u32) -> (f64, f64) {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as usize;
        let area = areas[seed % areas.len()];
        let max_x = area.2.saturating_sub(width);
        let max_y = area.3.saturating_sub(height);
        let x = (seed / areas.len()) % (max_x as usize + 1);
        let y = (seed / (areas.len() + 1)) % (max_y as usize + 1);
        (f64::from(area.0 + x as i32), f64::from(area.1 + y as i32))
    }
}

impl ApplicationHandler for Bouncer {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let initial_area = (
            self.position.0 as i32,
            self.position.1 as i32,
            self.atlas.width,
            self.atlas.height,
        );
        let created = platform::make_window(event_loop, initial_area);
        let mut renderer = LayeredRenderer::new(created.hwnd, self.atlas.width, self.atlas.height);
        if let Some(frame) = self.atlas.frame(0) {
            renderer.present(frame);
        }
        self.renderer = Some(renderer);
        self.window = Some(created);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => self.redraw(),
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.advance(event_loop);
    }
}
