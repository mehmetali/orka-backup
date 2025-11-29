use makepad_widgets::*;
use std::path::Path;
use crate::run_app;
use std::path::PathBuf;
use std::fs;
use makepad_widgets::resource;

// Makepad resource loader override: fontları exe'nin çalıştığı klasörden yükle
#[cfg(target_os = "windows")]
pub fn set_local_resource_loader() {


    fn load_resource_local(resource_path: &str) -> Option<Vec<u8>> {
        let exe_dir = std::env::current_exe().ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."));
        let local_path = exe_dir.join(resource_path);
        fs::read(&local_path).ok()
    }

    resource::set_resource_loader(Box::new(load_resource_local));
}
pub fn app_main() {
    #[cfg(target_os = "windows")]
    set_local_resource_loader();
    App;
}
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
        makepad_widgets::live_design(cx);
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