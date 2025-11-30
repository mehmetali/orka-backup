use makepad_widgets::*;
use std::path::Path;
use crate::run_app;

#[cfg(target_os = "windows")]
use tray_item::{TrayItem, IconSource, Menu, TrayEvent};
#[cfg(target_os = "windows")]
use std::sync::mpsc::Receiver;

live_design! {
    use makepad_widgets::theme_desktop_dark::*;
    use makepad_widgets::makepad_widgets::*;
    App = {{App}} {
        ui: <Root> {
            main_window = <Window> {
                window: {title: "MSSQL Backup Service"},
                body = <View> {
                    width: Fill,
                    height: Fill,
                    align: {x: 0.5, y: 0.5},
                    spacing: 20,
                    setup_button = <Button> { text: "Setup" }
                    log_button = <Button> { text: "View Logs" }
                    quit_button = <Button> { text: "Hide" }
                }
            }
        }
    }
}

#[derive(Live, LiveHook)]
pub struct App {
    #[live]
    ui: WidgetRef,
    #[cfg(target_os = "windows")]
    #[rust]
    tray_item: Option<TrayItem>,
    #[cfg(target_os = "windows")]
    #[rust]
    tray_receiver: Option<Receiver<TrayEvent>>,
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
            window.minimize(cx);
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

        if let Event::WindowCloseRequested(event) = event {
            if event.window_id == self.ui.window(&[id!(main_window)]).window_id() {
                self.ui.window(&[id!(main_window)]).minimize(cx);
            }
        }

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

            #[cfg(target_os = "windows")]
            {
                let (sender, receiver) = std::sync::mpsc::channel();

                let width = 16;
                let height = 16;
                let mut icon_data = Vec::with_capacity((width * height * 4) as usize);
                for _ in 0..(width * height) {
                    icon_data.extend_from_slice(&[255, 0, 0, 255]); // Red pixel (R, G, B, A)
                }

                let mut tray = TrayItem::new(
                    "MSSQL Backup Service",
                    IconSource::Raw{data: icon_data, width, height}
                ).expect("Failed to create tray item");

                tray.set_event_sender(sender);

                let mut menu = Menu::new();
                menu.add_item("Show", || {});
                menu.add_separator();
                menu.add_item("Quit", || {});

                tray.set_menu(&menu);

                self.tray_item = Some(tray);
                self.tray_receiver = Some(receiver);

                self.ui.window(&[id!(main_window)]).minimize(cx);
            }
        }

        #[cfg(target_os = "windows")]
        if let Some(receiver) = &self.tray_receiver {
            if let Ok(event) = receiver.try_recv() {
                match event {
                    TrayEvent::MenuItemClick { id, .. } => {
                        let window = self.ui.window(&[id!(main_window)]);
                        match id.as_str() {
                            "Show" => {
                                window.restore(cx);
                            },
                            "Quit" => {
                                cx.quit();
                            },
                            _ => {}
                        }
                    },
                    _ => {}
                }
            }
        }
    }
}

app_main!(App);
