use makepad_widgets::*;
use std::path::Path;
use crate::run_app;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
/*
    App = {{App}} {
        ui: <Window> {
            show_bg: true,
            width: Fit,
            height: Fit,
            body = <View> {
                align: {x: 0.5, y: 0.5},
                spacing: 20,
                setup_button: <Button> { text: "Setup" },
                log_button: <Button> { text: "View Logs" },
                quit_button: <Button> { text: "Quit" }
            }
        }
    }
        */
    App = {{App}} {
        ui: <Root>{
            main_window = <Window>{
                window: {title: "Hello"},
                body = <View> {
                    padding: 100,
                    <View> {
                        width: 300,
                        height: 750,
                        flow: Right {
                            row_align: Bottom,
                            wrap: true,
                        },
                        show_bg: true,
                        draw_bg: {
                            color: #888
                        }
                        <Button> {
                            margin: 0.0,
                            width: 100,
                            height: 100,
                            metrics: {
                                descender: 50,
                            }
                        }
                        <Button> {
                            margin: 0.0,
                            width: 200,
                            height: 200,
                        }
                        <Button> {
                            margin: 0.0,
                            width: 200,
                            height: 200,
                            metrics: {
                                descender: 100.0,
                                line_scale: 1.1,
                            }
                        }
                        <Button> {
                            margin: 0.0,
                            width: 100,
                            height: 100,
                        }
                    }
                }
            }
        }
    }
}

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
}

impl LiveRegister for App {
    fn live_register(cx: &mut Cx) {
        tracing::info!("Makepad live register started.");
        makepad_widgets::live_design(cx);
        live_design!{
            tracing::info!("Makepad live register macro started.");
            makepad_widgets::makepad_draw::shader::std::font_atlas::font_sdf;
            makepad_widgets::makepad_draw::font_loader::Font;

            THEME_FONT_REGULAR = {
                font_family:{
                    latin = {path: dep("IBMPlexSans-Text.ttf")},
                    chinese = {path: dep("LXGWWenKaiRegular.ttf")},
                    emoji = {path: dep("NotoColorEmoji.ttf")},
                }
            }
            THEME_FONT_BOLD = {
                font_family:{
                    latin = {path: dep("IBMPlexSans-SemiBold.ttf")},
                    chinese = {path: dep("LXGWWenKaiRegular.ttf")},
                    emoji = {path: dep("NotoColorEmoji.ttf")},
                }
            }
            THEME_FONT_ITALIC = {
                font_family:{
                    latin = {path: dep("IBMPlexSans-Italic.ttf")},
                    chinese = {path: dep("LXGWWenKaiRegular.ttf")},
                    emoji = {path: dep("NotoColorEmoji.ttf")},
                }
            }
        }
    }
}

impl MatchEvent for App {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        if self.ui.button(&[id!(quit_button)]).clicked(actions) {
            cx.quit();
        }
        if self.ui.button(&[id!(setup_button)]).clicked(actions) {
            log!("Setup button clicked!");
        }
        if self.ui.button(&[id!(log_button)]).clicked(actions) {
            log!("Log button clicked!");
        }
    }
}

impl AppMain for App {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event) {
        tracing::info!("Makepad event loop started.");
        tracing::info!("AppMain::handle_event received event: {:?}", event);
        self.match_event(cx, event);
        self.ui.handle_event(cx, event, &mut Scope::empty());

        if let Event::Startup = event {
            if Path::new("config.toml").exists() {
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    if let Err(e) = rt.block_on(run_app()) {
                        tracing::error!("Backup thread failed: {}", e);
                    }
                });
            } else {
                log!("Config file not found. Please set up the application.");
            }
        }
    }
}

app_main!(App);
