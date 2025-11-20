use crate::{
    assets::icons,
    service::music_service::{self, core::Core, models::PlayState},
    ui::modules::button::Button,
    utils::utils,
};
use gpui::{
    AsyncApp, ClickEvent, Context, ExternalPaths, ImageSource, SharedString, Task, WeakEntity,
    Window, div, img, prelude::*, px, relative, rgb, svg,
};
use std::time::Duration;

pub struct MyApp {
    music_core: music_service::core::Core,
    refresh_task: Option<Task<()>>,
    vol: f32,
}

impl MyApp {
    /// Init app struct
    pub fn init() -> Self {
        Self {
            music_core: Core::new(),
            refresh_task: None,
            vol: 1.0,
        }
    }

    /// Get current player status
    fn current_status(&mut self) -> SharedString {
        let text = match self.music_core.get_state() {
            PlayState::Playing => "PLAYING",
            PlayState::Stopped => "IDLEING",
            PlayState::Paused => "PAUSED",
        };
        format!("NOW {}", text).into()
    }

    /// Get name of current song
    fn current_name(&self) -> SharedString {
        if let Some(music) = &self.music_core.current {
            if let Some(tags) = music.get_tags() {
                if let Some(title) = tags.get_string(&lofty::tag::ItemKey::TrackTitle) {
                    return SharedString::new(title);
                }
            }
        }
        SharedString::new("-")
    }

    /// Get cover picture of current song
    fn current_picture(&self) -> Option<ImageSource> {
        if let Some(music) = &self.music_core.current {
            if let Some(tags) = music.get_tags() {
                if let Some(pic) = tags.pictures().first() {
                    if let Some(src) = utils::convert_picture(pic) {
                        return Some(src);
                    }
                }
            }
        }
        None
    }

    fn current_process(&self) -> f32 {
        if let Some(p) = self.music_core.player.as_ref() {
            let played = match p.played_time() {
                Some(t) => t.seconds,
                None => 0,
            };
            let total = match p.duration() {
                Some(t) => t.seconds,
                None => 0,
            };
            return played as f32 / total as f32;
        }
        0.
    }

    /// File deop event
    fn handle_file_drop(
        &mut self,
        event: &ExternalPaths,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // try get path
        if let Some(path) = event.paths().first() {
            // check whether file
            if !path.is_file() {
                return;
            }
            // append to player
            if let Err(e) = self.music_core.append(path.clone()) {
                eprintln!("error: {}", e);
            }
            // start refresh page
            self.spawn_refresh(cx);
            // update view
            cx.notify();
        }
    }

    /// Switch player state
    fn handle_switch_player(
        &mut self,
        _: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match self.music_core.get_state() {
            PlayState::Playing => {
                self.music_core.pause();
                self.refresh_task = None;
            }
            PlayState::Paused => {
                self.music_core.play();
                self.spawn_refresh(cx);
            }
            PlayState::Stopped => (),
        }
        cx.notify();
    }

    /// spawn a refresh task to refresh indicater during playing
    fn spawn_refresh(&mut self, _cx: &mut Context<Self>) {
        let t = _cx.spawn(
            async move |app_weak: WeakEntity<MyApp>, cx: &mut AsyncApp| {
                loop {
                    if let Some(app) = app_weak.upgrade() {
                        if let Err(_) =
                            app.update(cx, |app: &mut MyApp, _cx: &mut Context<Self>| {
                                if let Some(p) = app.music_core.player.as_ref() {
                                    if p.state() == PlayState::Stopped && p.occupied_len() == 0 {
                                        app.drop_core(_cx);
                                    }
                                    _cx.notify();
                                }
                            })
                        {
                            return;
                        }
                    }
                    cx.background_executor()
                        .timer(Duration::from_millis(100))
                        .await;
                }
            },
        );
        self.refresh_task = Some(t);
    }

    fn drop_core(&mut self, cx: &mut Context<Self>) {
        self.music_core.stop();
        self.refresh_task = None;
        cx.notify();
    }

    fn handle_drop_core(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        self.drop_core(cx);
    }

    fn handle_switch_volume(&mut self, _: &ClickEvent, _: &mut Window, _: &mut Context<Self>) {
        if self.vol >= 1.0 {
            self.vol = 0.0;
        } else {
            self.vol += 0.2;
        }
        println!("new volume {}", self.vol);
        self.music_core.set_gain(self.vol);
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
                    .relative()
                    .w_full()
                    .h_2_3()
                    .bg(rgb(0x398ad7))
                    .text_color(gpui::white())
                    .flex()
                    .flex_col()
                    .justify_center()
                    .items_center()
                    .child(div().text_xl().child(self.current_status()))
                    .child(if self.current_picture().is_none() {
                        div()
                    } else {
                        div().child(
                            img(self.current_picture().unwrap())
                                .size(px(150.0))
                                .rounded_md(),
                        )
                    })
                    .child(div().text_3xl().child(self.current_name()))
                    .child(if let Some(p) = self.music_core.player.as_ref() {
                        format!(
                            "{} / {}",
                            utils::format_time(p.played_time().unwrap()),
                            utils::format_time(p.duration().unwrap()),
                        )
                    } else {
                        "".to_string()
                    })
                    .child(
                        div()
                            .absolute()
                            .bottom_0()
                            .bg(rgb(0x88b7e7))
                            .h_1p5()
                            .left_0()
                            .w(relative(self.current_process())),
                    ),
            )
            .child(
                div()
                    .gap_5()
                    .w_full()
                    .h_1_3()
                    .bg(gpui::white())
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(
                        Button::new("volume")
                            .child(
                                svg()
                                    .path(match self.vol {
                                        0.0 => icons::VOLUME_MUTE,
                                        1.0 => icons::VOLUME_UP,
                                        _ => icons::VOLUME_DOWN,
                                    })
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .text_color(gpui::white()),
                            )
                            .on_click(_cx.listener(Self::handle_switch_volume)),
                    )
                    .child(
                        Button::new("button_play_pause")
                            .on_click(_cx.listener(Self::handle_switch_player))
                            .child(
                                svg()
                                    .path(icons::PLAY_PAUSE_FILLED)
                                    .w(px(32.0))
                                    .h(px(32.0))
                                    .text_color(gpui::white()),
                            ),
                    )
                    .child(
                        Button::new("button_stop")
                            .child(
                                svg()
                                    .path(icons::STOP_FILLED)
                                    .w(px(26.0))
                                    .h(px(26.0))
                                    .text_color(gpui::white()),
                            )
                            .on_click(_cx.listener(Self::handle_drop_core)),
                    ),
            )
    }
}
