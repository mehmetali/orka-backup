use makepad_widgets::*;
use std::path::Path;
use crate::run_app;

live_design! {
    import makepad_widgets::base::*;
    import makepad_widgets::theme_desktop_dark::*;

    App = {{App}} {
        ui: <Window> {
            show_bg: true,
            width: Fit,
            height: Fit,
            body = <View> {
                align: {x: 0.5, y: 0.5},
                spacing: 20,
                setup_button = <Button> { text: "Setup" }
                log_button = <Button> { text: "View Logs" }
                quit_button = <Button> { text: "Quit" }
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
        if self.ui.button(id!(quit_button)).clicked(actions) {
            cx.quit();
        }
        if self.ui.button(id!(setup_button)).clicked(actions) {
            log!("Setup button clicked!");
        }
        if self.ui.button(id!(log_button)).clicked(actions) {
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

