use crate::service::music_service::{
    self,
    core::{self, Core},
    music::Music,
};
use gpui::{
    ClickEvent, Context, ExternalPaths, Image, ImageFormat, ImageSource, SharedString, Window, div,
    img, prelude::*, px, rgb, svg,
};
use lofty::picture::Picture;
use std::{path::PathBuf, sync::Arc};

enum PlayStatus {
    Playing,
    Idleing,
    Pausing,
}

pub struct MyApp {
    music_core: music_service::core::Core,
    status: PlayStatus,
    song_name: SharedString,
    song_picture: Option<ImageSource>,
    status_text: SharedString,
}

impl MyApp {
    pub fn init() -> Self {
        Self {
            music_core: Core::new(),
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
                return Some(ImageSource::Image(Arc::new(Image::from_bytes(
                    mtype.unwrap(),
                    pic.data().to_vec(),
                ))));
            }
        }
        None
    }

    fn get_music_meta(&mut self) {
        if let Some(music) = &self.music_core.current {
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

    fn load_new_music(&mut self, path_str: PathBuf) {
        // self.music_player.append_music(&music).unwrap();
        self.music_core.append(path_str);
        self.get_music_meta();
        self.set_status(PlayStatus::Playing);
    }

    fn handle_file_drop(
        &mut self,
        event: &ExternalPaths,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(path) = event.paths().first() {
            if !path.is_file() {
                return;
            }
            self.load_new_music(path.clone());
        }
        cx.notify();
    }

    fn handle_switch_player(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.status {
            PlayStatus::Playing => {
                self.music_core.pause();
                self.set_status(PlayStatus::Pausing);
            }
            PlayStatus::Pausing => {
                self.music_core.play();
                self.set_status(PlayStatus::Playing);
            }
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
                    .child(div().text_xl().child(self.status_text.clone()))
                    .child(if self.song_picture.is_none() {
                        div()
                    } else {
                        div().child(
                            img(self.song_picture.as_ref().unwrap().clone())
                                .size(px(150.0))
                                .rounded_md(),
                        )
                    })
                    .child(div().text_3xl().child(self.song_name.clone())),
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
                                svg()
                                    .path("icons/play_pause.svg")
                                    .w(px(32.0))
                                    .h(px(32.0))
                                    .text_color(gpui::white()),
                            )
                            .hover(|style| style.bg(rgb(0x98acc1)))
                            .on_click(_cx.listener(Self::handle_switch_player)),
                    ),
            )
    }
}
