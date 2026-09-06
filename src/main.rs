#![cfg_attr(windows, windows_subsystem = "windows")]

mod assets;
mod bouncer;
mod config;
mod platform;
mod renderer;
mod tray;

use bouncer::Bouncer;
use rfd::{MessageButtons, MessageDialog, MessageLevel};
use std::path::Path;
use winit::event_loop::EventLoop;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    platform::set_process_dpi_awareness();
    let config = config::read()?;
    let animation_path = Path::new(config::ANIMATION_FILE);
    if !animation_path.exists() {
        MessageDialog::new()
            .set_level(MessageLevel::Error)
            .set_title("Sinonome Bouncer")
            .set_description(format!(
                "必要なPNGファイルが見つかりません。\n{}",
                animation_path.display()
            ))
            .set_buttons(MessageButtons::Ok)
            .show();
        return Ok(());
    }
    let atlas = match assets::load_png_tiles(
        animation_path,
        config.frame_count,
        config.scale,
        config.frame_delays,
    ) {
        Ok(atlas) => atlas,
        Err(error) => {
            MessageDialog::new()
                .set_level(MessageLevel::Error)
                .set_title("Sinonome Bouncer")
                .set_description(format!("PNGの読み込みに失敗しました。\n{error}"))
                .set_buttons(MessageButtons::Ok)
                .show();
            return Ok(());
        }
    };
    let area = platform::virtual_work_area(config.monitor)?;
    let (tray_icon, quit_item_id) = tray::setup(&atlas)?;
    let event_loop = EventLoop::new()?;
    let mut app = Bouncer::new(atlas, area, config.bounce_speed, tray_icon, quit_item_id);
    event_loop.run_app(&mut app)?;
    Ok(())
}
