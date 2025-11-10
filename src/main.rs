mod service;
mod assets;
mod ui;
use ui::app::MyApp;

use gpui::{
    App, Application, Bounds, SharedString, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size
};

use crate::service::music_core;

 
fn main() {
    // Application::new()
    //     .with_assets(assets::assets::Assets)
    //     .run(|cx: &mut App| {
    //     let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
    //     cx.open_window(
    //         WindowOptions {
    //             window_bounds: Some(WindowBounds::Windowed(bounds)),
    //             titlebar: Some(TitlebarOptions {
    //                 title: Some(SharedString::new("The Player")),
    //                 appears_transparent: false,
    //                 traffic_light_position: None,
                    
    //             }),
    //             ..Default::default()
    //         },
    //         |_, cx| {
    //             cx.new(|_| MyApp::init())
    //         },
    //     )
    //     .unwrap();
    // });


    music_core::MusicCore::some();
}