use makepad_widgets::*;
use std::path::Path;
use crate::run_app;


live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;
    App = {{App}} {
        ui: <Root> {
            main_window = <Window> {
                window: {title: "MSSQL Backup Service"},
                body = <View> {
                    setup_button = <Button> { text: "Setup" }
                    log_button = <Button> { text: "View Logs" }
                    quit_button = <Button> { text: "Quit" }
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
       let window = self.ui.window(&[id!(main_window)]);
        if self.ui.button(&[id!(quit_button)]).clicked(actions) {
            println!("{:?}", window);
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

        if let Event::WindowCloseRequested(_event) = event {
            // Can't access window_id on WindowRef in this makepad version; handle close by targeting main window directly
            if let Some(_window) = self.ui.window(&[id!(main_window)]).borrow_mut() {
                tracing::info!("WindowCloseRequested: would minimize to tray (no-op)");
            }
        }

    }
}

app_main!(App);
