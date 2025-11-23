use std::time::Duration;

use crate::{
    assets::icons,
    service::music_service::{self, core::Core, models::PlayState},
    ui::modules::button::Button,
    utils::utils,
};
use gpui::{
    AsyncApp, ClickEvent, Context, ExternalPaths, ImageSource, MouseDownEvent, SharedString, Task,
    WeakEntity, Window, div, img, prelude::*, px, relative, rgb, rgba, svg,
};
use symphonia::core::units::Time;

pub struct MyApp {
    music_core: music_service::core::Core,
    refresh_task: Option<Task<()>>,
    volume: f32,
    message: String,
    msg_timer: Option<Task<()>>,
}

impl MyApp {
    /// Init app struct
    pub fn init() -> Self {
        Self {
            music_core: Core::new(),
            refresh_task: None,
            volume: 1.0,
            message: "".into(),
            msg_timer: None,
        }
    }

    fn show_msg(&mut self, cx: &mut Context<Self>, msg: String, duration: Duration) {
        self.message = msg;
        cx.notify();

        self.msg_timer = Some(
            cx.spawn(async move |weak: WeakEntity<MyApp>, cx: &mut AsyncApp| {
                cx.background_executor().timer(duration).await;

                weak.update(cx, |app, cx| {
                    app.message = "".into();
                    cx.notify();
                })
                .unwrap();
            }),
        );
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
        if let Some(music) = &self.music_core.current() {
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
        let Some(music) = self.music_core.current() else {
            return None;
        };
        let Some(tags) = music.get_tags() else {
            return None;
        };
        let Some(pic) = tags.pictures().first() else {
            return None;
        };
        let Some(src) = utils::convert_picture(pic) else {
            return None;
        };
        Some(src)
    }

    /// Get current play progress
    fn current_progress(&self) -> f32 {
        if let Some(p) = self.music_core.player() {
            return p.play_time().played_sec() as f32 / p.play_time().duration_sec() as f32;
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
                self.show_msg(cx, format!("Error: {}", e), Duration::from_secs(6));
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
        let Some(p) = self.music_core.player() else {
            return;
        };
        let rx = p.receiver();

        let t = _cx.spawn(async move |weak: WeakEntity<MyApp>, cx: &mut AsyncApp| {
            while let Ok(e) = rx.recv().await {
                let r = weak.update(cx, |app, cx| {
                    match e {
                        music_service::models::Events::PlaytimeRefresh => (),
                        music_service::models::Events::PlayFinished => app.music_core.stop(),
                    };
                    cx.notify();
                });
                if let Err(_) = r {
                    break;
                }
            }
        });
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

    fn handle_switch_volume(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        if self.volume >= 1.0 {
            self.volume = 0.0;
        } else {
            self.volume += 0.2;
        }
        self.show_msg(
            cx,
            format!("Volume {}%", (self.volume * 100.0) as u32),
            Duration::from_secs(2),
        );
        self.music_core.set_gain(self.volume);
    }

    fn handle_process_click(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(p) = self.music_core.player() {
            let per = event.position.x.to_f64() / window.viewport_size().width.to_f64();
            let time_point = (p.play_time().duration_sec() as f64 * per + 0.5).floor();
            p.seek_to(Time::from(time_point));
            cx.notify();
        }
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
                    .text_align(gpui::TextAlign::Center)
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
                    .child(if let Some(p) = self.music_core.player() {
                        format!(
                            "{} / {}",
                            utils::format_time(p.play_time().played_sec()),
                            utils::format_time(p.play_time().duration_sec()),
                        )
                    } else {
                        "".to_string()
                    })
                    .child(
                        div()
                            .id("processer")
                            .absolute()
                            .bottom_0()
                            .w_full()
                            // .bg(rgb(0x232323))
                            .on_mouse_down(
                                gpui::MouseButton::Left,
                                _cx.listener(Self::handle_process_click),
                            )
                            .child(
                                div()
                                    .bg(rgba(0xffffff66))
                                    .h_1p5()
                                    .left_0()
                                    .w(relative(self.current_progress())),
                            ),
                    ),
            )
            .child(
                div()
                    .relative()
                    .gap_5()
                    .w_full()
                    .h_1_3()
                    .bg(gpui::white())
                    .flex()
                    .justify_center()
                    .items_center()
                    .child(
                        div()
                            .absolute()
                            .top_1p5()
                            .text_align(gpui::TextAlign::Center)
                            .text_color(rgb(0x323232))
                            .text_sm()
                            .child(self.message.clone()),
                    )
                    .child(
                        Button::new("volume")
                            .child(
                                svg()
                                    .path(match self.volume {
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
