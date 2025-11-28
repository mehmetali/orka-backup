use fltk::{
    app,
    button::Button,
    frame::Frame,
    prelude::*,
    window::Window,
    input::{Input, SecretInput},
};
use anyhow::Result;
use fltk::dialog;
use fltk_theme::{ThemeType, WidgetTheme};
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use crate::config::{ApiConfig, BackupConfig, Config, MssqlConfig};
use fltk::{text::{TextBuffer, TextDisplay}};
use std::fs;
use crate::logging;

pub fn show_log_window() -> Result<()> {
    let mut wind = Window::new(100, 100, 600, 400, "Logs");
    let mut text_buffer = TextBuffer::default();
    let mut text_display = TextDisplay::new(5, 5, 590, 390, "");
    wind.make_resizable(true);
    wind.resizable(&text_display);

    let log_path = logging::get_log_filepath();

    let log_content = match fs::read_to_string(&log_path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            "Log file has not been created yet.".to_string()
        }
        Err(e) => format!("Failed to read log file: {}", e),
    };

    text_buffer.set_text(&log_content);
    text_display.set_buffer(text_buffer.clone());

    wind.end();
    wind.show();

    let (s, r) = app::channel();

    // Initial load
    update_log_view(&mut text_display, &mut text_buffer, &log_path);

    std::thread::spawn(move || {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
            s.send(());
        }
    });

    while wind.shown() {
        if app::wait_for(0.1).unwrap() {
            if let Some(_) = r.recv() {
                update_log_view(&mut text_display, &mut text_buffer, &log_path);
            }
        }
    }

    Ok(())
}

fn update_log_view(
    text_display: &mut TextDisplay,
    text_buffer: &mut TextBuffer,
    log_path: &std::path::Path,
) {
    let log_content = match fs::read_to_string(log_path) {
        Ok(content) => content,
        Err(_) => return,
    };

    let current_content = text_buffer.text();
    if current_content != log_content {
        text_buffer.set_text(&log_content);
        text_display.scroll(
            text_buffer.length() as i32,
            0,
        );
    }
}


pub fn show_setup_window() -> Result<bool> {
    let app = app::App::default();
    let widget_theme = WidgetTheme::new(ThemeType::Aero);
    widget_theme.apply();
    let mut wind = Window::new(100, 100, 400, 400, "Setup");
    let _frame = Frame::new(0, 0, 400, 50, "Enter Configuration");

    let api_url_input = Input::new(150, 60, 200, 25, "API URL:");
    let server_token_input = Input::new(150, 90, 200, 25, "Server Token:");
    let auth_token_input = SecretInput::new(150, 120, 200, 25, "Auth Token:");
    let temp_path_input = Input::new(150, 150, 200, 25, "Temp Path:");
    let db_host_input = Input::new(150, 180, 200, 25, "DB Host:");
    let db_port_input = Input::new(150, 210, 200, 25, "DB Port:");
    let db_user_input = Input::new(150, 240, 200, 25, "DB User:");
    let db_pass_input = SecretInput::new(150, 270, 200, 25, "DB Pass:");
    let db_name_input = Input::new(150, 300, 200, 25, "DB Name:");
    let db_instance_input = Input::new(150, 330, 200, 25, "DB Instance:");

    let mut save_button = Button::new(150, 370, 100, 30, "Save");
    wind.end();
    wind.show();

    let saved = Arc::new(Mutex::new(false));
    let saved_clone = saved.clone();

    save_button.set_callback(move |_| {
        let port_str = db_port_input.value();
        if !port_str.is_empty() && port_str.parse::<u16>().is_err() {
            dialog::alert_default("Invalid port number.");
            return;
        }

        let host = db_host_input.value();
        let port = db_port_input.value();
        let user = db_user_input.value();
        let pass = db_pass_input.value();
        let instance_name = db_instance_input.value();

        let config = Config {
            mssql: MssqlConfig {
                host: if host.is_empty() { None } else { Some(host) },
                port: if port.is_empty() { None } else { port.parse().ok() },
                user: if user.is_empty() { None } else { Some(user) },
                pass: if pass.is_empty() { None } else { Some(pass) },
                database: db_name_input.value(),
                instance_name: if instance_name.is_empty() { None } else { Some(instance_name) },
            },
            api: ApiConfig {
                url: api_url_input.value(),
                server_token: server_token_input.value(),
                auth_token: auth_token_input.value(),
            },
            backup: BackupConfig {
                temp_path: temp_path_input.value(),
            },
        };

        match (|| -> Result<()> {
            let toml = toml::to_string(&config)?;
            let mut file = File::create("config.toml")?;
            file.write_all(toml.as_bytes())?;
            Ok(())
        })() {
            Ok(_) => {
                *saved_clone.lock().unwrap() = true;
                app.quit();
            }
            Err(e) => {
                dialog::alert_default(&format!("Failed to save config: {}", e));
            }
        }
    });

    app.run()?;
    let result = *saved.lock().unwrap();
    Ok(result)
}
