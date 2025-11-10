mod service;
mod assets;
use std::sync::Arc;

use gpui::{
    App, Application, Bounds, ClickEvent, Context, ExternalPaths, Image, ImageFormat, ImageSource, SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions, div, img, prelude::*, px, rgb, size, svg
};
use lofty::picture::Picture;
use crate::service::{music::Music, music_player::MusicService};

enum PlayStatus {
    Playing,
    Idleing,
    Pausing,
}
 
struct MyApp {
    music_player: MusicService,
    current_music: Option<Music>,
    status: PlayStatus,
    song_name: SharedString,
    song_picture: Option<ImageSource>,
    status_text: SharedString,
}

impl MyApp {
    fn init() -> Self {
        Self { 
            music_player: MusicService::new().unwrap(), 
            current_music: None, 
            status: PlayStatus::Idleing,
            song_name: "-".into(), 
            song_picture: None,
            status_text: "NOW IDLEING".into(),
        }
    }

    // fn set_status(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
    fn set_status(&mut self, new_status: PlayStatus) {
        let text = match new_status {
            PlayStatus::Playing => "PLAYING",
            PlayStatus::Idleing => "IDLEING",
            PlayStatus::Pausing => "PAUSING",
        };
        self.status = new_status;
        self.status_text = format!("NOW {}", text).into();
    }

    fn get_picture(&self, pic: &Picture) -> Option<ImageSource> {
        if let Some(mime) = pic.mime_type() {
            let mtype = match mime {
                lofty::picture::MimeType::Png => Some(ImageFormat::Png),
                lofty::picture::MimeType::Jpeg => Some(ImageFormat::Jpeg),
                lofty::picture::MimeType::Tiff => Some(ImageFormat::Tiff),
                lofty::picture::MimeType::Bmp => Some(ImageFormat::Bmp),
                lofty::picture::MimeType::Gif => Some(ImageFormat::Gif),
                _ => None,
            };
            if mtype != None {
                return Some(ImageSource::Image(
                    Arc::new(Image::from_bytes(mtype.unwrap(), pic.data().to_vec()))
                ))
            }
        }
        None
    }

    fn get_music_meta(&mut self) {
        if let Some(music) = &self.current_music {
            if let Some(tags) = music.get_tags() {
                if let Some(title) = tags.get_string(&lofty::tag::ItemKey::TrackTitle) {
                    self.song_name = SharedString::new(title);
                }
                if let Some(pic) = tags.pictures().first() {
                    // self.song_picture = Some(pic.clone());
                    if let Some(src) = self.get_picture(pic) {
                        self.song_picture = Some(src);
                    }
                }
            }
        }
    }

    fn load_new_music(&mut self, music: Music) {
        self.music_player.append_music(&music).unwrap();
        self.current_music = Some(music);
        self.get_music_meta();
        self.set_status(PlayStatus::Playing);
    }
    
    fn handle_file_drop(&mut self, event: &ExternalPaths, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = event.paths().first() {
            if !path.is_file() {
                return;
            }
            if let Ok(music) = Music::from_path(path) {
                self.load_new_music(music);
            }
        }
        cx.notify();
    }

    fn handle_switch_player(&mut self, event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>){
        match self.status {
            PlayStatus::Playing => {
                self.music_player.pause();
                self.set_status(PlayStatus::Pausing);
            },
            PlayStatus::Pausing => {
                self.music_player.play();
                self.set_status(PlayStatus::Playing);
            },
            PlayStatus::Idleing => (),
        }
        cx.notify();
    }
}
 
impl Render for MyApp {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h_full()
            .on_drop(_cx.listener(Self::handle_file_drop))
            .child(
                div()
                    .id("drop-target")
                    .w_full()
                    .h_2_3()
                    .bg(rgb(0x398ad7))
                    .text_color(gpui::white())
                    .flex()
                    .flex_col()
                    .justify_center()
                    .items_center()
                    .child(
                        div()
                            .text_xl()
                            .child(self.status_text.clone())
                    )
                    
                    .child(
                        if self.song_picture.is_none() {
                            div()
                        } else {
                            div()
                                .child(
                                    img(self.song_picture.as_ref().unwrap().clone())
                                        .size(px(150.0))
                                        .rounded_md()
                                    )
                        }
                    )
                    .child(
                        div()
                            .text_3xl()
                            .child(self.song_name.clone())
                    )
            )
            .child(
                div()
                    .w_full()
                    .h_1_3()
                    .bg(gpui::white())
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(
                        div()
                            .id("click_area")
                            // .border_1()
                            // .border_color(gpui::black())
                            .rounded_3xl()
                            .bg(rgb(0x88b7e7))
                            .w_16()
                            .h_16()
                            .flex()
                            .justify_center()
                            .items_center()
                            .text_color(gpui::white())
                            .child(
                                svg().path("icons/play_pause.svg")
                            )
                            .hover(|style| style.bg(rgb(0xdee2e6)))
                            .on_click(_cx.listener(Self::handle_switch_player))
                    )
            )
    }
}
 
fn main() {
    Application::new()
        .with_assets(assets::assets::Assets)
        .run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(500.), px(500.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::new("The Player")),
                    appears_transparent: false,
                    traffic_light_position: None,
                    
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| MyApp::init())
            },
        )
        .unwrap();
    });
}