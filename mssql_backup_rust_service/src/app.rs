use makepad_widgets::*;
use std::path::Path;
use crate::run_app;

#[cfg(target_os = "windows")]
use tray_item::{TrayItem, IconSource};
#[cfg(target_os = "windows")]
use std::sync::mpsc::Receiver;

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
        if let Some(mut window) = self.ui.window(&[id!(main_window)]).borrow_mut() {
            window.window(_).minimize(cx);
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

        if let Event::WindowCloseRequested(event) = event {
            if event.window_id == self.ui.window(&[id!(main_window)]).window_id() {
                if let Some(mut window) = self.ui.window(&[id!(main_window)]).borrow_mut() {
                    window.window.minimize(cx);
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            let (sender, receiver) = std::sync::mpsc::channel();
            
            let icon_data = vec![0, 0, 0, 0];
            let mut tray = TrayItem::new(
                "MSSQL Backup Service",
                IconSource::Raw{data: icon_data, width: 1, height: 1}
            ).expect("Failed to create tray item");

            tray.set_event_sender(sender);
            
            let mut menu = Menu::new();
            menu.add_item("Show", || {});
            menu.add_separator();
            menu.add_item("Quit", || {});
            
            tray.set_menu(&menu);
            
            self.tray_item = Some(tray);
            self.tray_receiver = Some(receiver);

            if let Some(mut window) = self.ui.window(&[id!(main_window)]).borrow_mut() {
                window.window.minimize(cx);
            }
        }

       #[cfg(target_os = "windows")]
        if let Some(receiver) = &self.tray_receiver {
            if let Ok(event) = receiver.try_recv() {
                match event {
                    TrayEvent::MenuItemClick { id, .. } => {
                        if let Some(mut window) = self.ui.window(&[id!(main_window)]).borrow_mut() {
                            match id.as_str() {
                                "Show" => {
                                    window.window.restore(cx);
                                    window.window.focus(cx);
                                },
                                "Quit" => {
                                    cx.quit();
                                },
                                _ => {}
                            }
                        }
                    },
                    _ => {}
                }
            }
    }
    }
}

app_main!(App);
