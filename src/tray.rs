use crate::assets::{self, Atlas};
use tray_icon::{
    menu::{Menu, MenuId, MenuItem},
    TrayIcon, TrayIconBuilder,
};

pub fn setup(atlas: &Atlas) -> Result<(TrayIcon, MenuId), Box<dyn std::error::Error>> {
    let menu = Menu::new();
    let quit_item = MenuItem::new("終了", true, None);
    menu.append(&quit_item)?;
    let quit_id = quit_item.id().clone();
    let tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("sinonome_bouncer")
        .with_icon(assets::tray_icon(atlas)?)
        .build()?;
    Ok((tray_icon, quit_id))
}
