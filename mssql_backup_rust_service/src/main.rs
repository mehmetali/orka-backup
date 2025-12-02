#![windows_subsystem = "windows"]

use iced::{
    executor,
    widget::{button, column, container, row, scrollable, text, text_input},
    window, Alignment, Application, Command, Element, Length, Settings, Theme,
};
use std::{path::{Path, PathBuf}, thread, time::Duration, sync::{Arc, Mutex}, fs};
use ctor::ctor;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tracing_subscriber::{prelude::*, EnvFilter};
use once_cell::sync::Lazy;
use anyhow::Result;
use tray_item::{TrayItem, IconSource};
use single_instance::SingleInstance;

mod config;
mod backup;
mod upload;
mod cleanup;
mod logging;
mod styling;


#[derive(Debug, Clone, serde::Deserialize)]
struct BackupEntry {
    id: u64,
    db_name: String,
    file_path: String,
    file_size_bytes: u64,
    backup_started_at: String,
    backup_completed_at: String,
    status: String,
}

async fn load_and_parse_logs() -> Result<Vec<LogEntry>, String> {
    let log_path = logging::get_log_filepath();
    let content = tokio::fs::read_to_string(log_path)
        .await
        .map_err(|e| e.to_string())?;

    let mut entries = Vec::new();
    for line in content.lines() {
        let parts: Vec<&str> = line.splitn(4, ' ').collect();
        if parts.len() >= 4 {
            let timestamp_utc_str = parts[0];
            let level = parts[2].to_string();
            let rest = parts[3];

            let module_end = rest.find(':').unwrap_or(0);
            let module = rest[..module_end].to_string();
            let message = rest[module_end + 1..].trim().to_string();

            let timestamp_utc =
                OffsetDateTime::parse(timestamp_utc_str, &Rfc3339).map_err(|e| e.to_string())?;
            let local_offset =
                time::UtcOffset::current_local_offset().map_err(|e| e.to_string())?;
            let timestamp_local = timestamp_utc.to_offset(local_offset);

            entries.push(LogEntry {
                timestamp: timestamp_local.to_string(),
                level,
                module,
                message,
            });
        }
    }
    Ok(entries)
}

static LOGGING_GUARD: Lazy<tracing_appender::non_blocking::WorkerGuard> = Lazy::new(init_logging);

#[ctor]
fn early_init() {
    Lazy::force(&LOGGING_GUARD);
    tracing::info!("Early init logging complete.");
}

fn get_show_file_path() -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push("mssql-backup-show-gui");
    path
}

fn main() {
    let instance = SingleInstance::new("mssql-backup-rust-service-unique-id").expect("Failed to create single instance");
    if instance.is_single() {
        // This is the first instance, run as service.
        run_service();
    } else {
        // Another instance is already running, notify it to show the GUI.
        if let Err(e) = fs::write(get_show_file_path(), "") {
            tracing::error!("Failed to write show GUI signal file: {}", e);
        }
    }
}

fn run_service() {
    let gui_thread_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>> = Arc::new(Mutex::new(None));

    // Listen for show requests via file
    let gui_handle_clone = Arc::clone(&gui_thread_handle);
    thread::spawn(move || {
        loop {
            let path = get_show_file_path();
            if path.exists() {
                show_gui(Arc::clone(&gui_handle_clone));
                if let Err(e) = fs::remove_file(path) {
                    tracing::error!("Failed to remove show GUI signal file: {}", e);
                }
            }
            thread::sleep(Duration::from_secs(1));
        }
    });


    // Spawn the tray icon thread
    let gui_handle_tray_clone = Arc::clone(&gui_thread_handle);
    thread::spawn(move || {
        let mut tray = TrayItem::new(
            "MSSQL Backup Service",
            IconSource::Resource("tray-default"),
        ).expect("Failed to create tray item");

        tray.add_label("MSSQL Backup Service").expect("Failed to add tray label");

        let inner_gui_handle = gui_handle_tray_clone;
        tray.add_menu_item("Show", move || {
            show_gui(Arc::clone(&inner_gui_handle));
        }).expect("Failed to add 'Show' menu item");

        tray.add_menu_item("Quit", || {
            std::process::exit(0);
        }).expect("Failed to add 'Quit' menu item");
    });

    // Spawn the background backup task
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    runtime.spawn(async {
        if let Err(e) = run_app().await {
            tracing::error!("Background backup task failed: {:?}", e);
        }
    });

    // Initially show the GUI
    show_gui(Arc::clone(&gui_thread_handle));

    // Keep the service alive
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

fn show_gui(gui_thread_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>) {
    let mut handle = gui_thread_handle.lock().expect("Failed to lock GUI thread handle");
    if handle.is_none() || handle.as_ref().unwrap().is_finished() {
        *handle = Some(thread::spawn(|| {
            if let Err(e) = App::run(Settings::default()) {
                tracing::error!("GUI thread failed: {:?}", e);
            }
        }));
    }
}
enum ViewState {
    Main,
    Settings,
    Logs,
    Backups,
}

#[derive(Debug, Clone)]
struct LogEntry {
    timestamp: String,
    level: String,
    module: String,
    message: String,
}

struct App {
    status: String,
    view_state: ViewState,
    config: config::Config,
    original_config: Option<config::Config>,
    logs: Vec<LogEntry>,
    backups: Vec<BackupEntry>,
}

#[derive(Debug, Clone)]
enum Message {
    Setup,
    ViewLogs,
    LogsLoaded(Result<Vec<LogEntry>, String>),
    BackToMain,
    Quit,
    StatusChanged(String),
    SaveConfig,
    Config(ConfigMessage),
    Cancel,
    ViewBackups,
    BackupsLoaded(Result<Vec<BackupEntry>, String>),
    DownloadBackup(u64),
    OpenUrl(String),
}

#[derive(Debug, Clone)]
pub enum ConfigMessage {
    HostChanged(String),
    PortChanged(String),
    UserChanged(String),
    PassChanged(String),
    DatabaseChanged(String),
    InstanceNameChanged(String),
    ApiUrlChanged(String),
    ServerTokenChanged(String),
    AuthTokenChanged(String),
    TempPathChanged(String),
}

impl Application for App {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        if Path::new("config.toml").exists() {
            match config::load_config("config.toml") {
                Ok(config) => {
                    let app = Self {
                        status: "Backup service running...".to_string(),
                        view_state: ViewState::Main,
                        original_config: Some(config.clone()),
                        config,
                        logs: vec![],
                        backups: vec![],
                    };
                    (app, Command::none()) // Background task is managed by the service now
                }
                Err(e) => {
                    let app = Self {
                        status: format!("Error loading config: {}", e),
                        view_state: ViewState::Settings,
                        config: config::Config::default(),
                        original_config: None,
                        logs: vec![],
                        backups: vec![],
                    };
                    (app, Command::none())
                }
            }
        } else {
            let app = Self {
                status: "Config file not found. Please set up the application.".to_string(),
                view_state: ViewState::Settings,
                config: config::Config::default(),
                original_config: None,
                logs: vec![],
                backups: vec![],
            };
            (app, Command::none())
        }
    }

    fn title(&self) -> String {
        String::from("MSSQL Backup Service")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Setup => {
                self.original_config = Some(self.config.clone());
                self.view_state = ViewState::Settings;
            }
            Message::ViewLogs => {
                return Command::perform(load_and_parse_logs(), Message::LogsLoaded);
            }
            Message::LogsLoaded(Ok(logs)) => {
                self.logs = logs;
                self.view_state = ViewState::Logs;
            }
            Message::LogsLoaded(Err(e)) => {
                self.status = format!("Error loading logs: {}", e);
            }
            Message::BackToMain => {
                self.view_state = ViewState::Main;
            }
            Message::Quit => {
                // This will close the window and the thread will exit.
                return window::close(window::Id::MAIN);
            }
            Message::StatusChanged(new_status) => {
                self.status = new_status;
            }
            Message::SaveConfig => {
                match config::save_config("config.toml", &self.config) {
                    Ok(_) => {
                        self.status = "Config saved successfully.".to_string();
                        self.view_state = ViewState::Main;
                        self.original_config = Some(self.config.clone());
                        // We might need to signal the service to restart the backup loop
                        // if config changes. For now, we just save and go to main.
                    }
                    Err(e) => {
                        self.status = format!("Error saving config: {}", e);
                    }
                }
            }
            Message::Config(config_message) => {
                match config_message {
                    ConfigMessage::HostChanged(s) => self.config.mssql.host = Some(s),
                    ConfigMessage::PortChanged(s) => self.config.mssql.port = s.parse().ok(),
                    ConfigMessage::UserChanged(s) => self.config.mssql.user = Some(s),
                    ConfigMessage::PassChanged(s) => self.config.mssql.pass = Some(s),
                    ConfigMessage::DatabaseChanged(s) => self.config.mssql.database = s,
                    ConfigMessage::InstanceNameChanged(s) => {
                        self.config.mssql.instance_name = Some(s)
                    }
                    ConfigMessage::ApiUrlChanged(s) => self.config.api.url = s,
                    ConfigMessage::ServerTokenChanged(s) => self.config.api.server_token = s,
                    ConfigMessage::AuthTokenChanged(s) => self.config.api.auth_token = s,
                    ConfigMessage::TempPathChanged(s) => self.config.backup.temp_path = s,
                }
            }
            Message::Cancel => {
                if let Some(original_config) = self.original_config.take() {
                    self.config = original_config;
                }
                self.view_state = ViewState::Main;
                self.status = "Editing cancelled.".to_string();
            }
            Message::ViewBackups => {
                self.view_state = ViewState::Backups;
                let config = self.config.clone();
                return Command::perform(fetch_backups(config), Message::BackupsLoaded);
            }
            Message::BackupsLoaded(Ok(backups)) => {
                self.backups = backups;
            }
            Message::BackupsLoaded(Err(e)) => {
                self.status = format!("Error loading backups: {}", e);
            }
            Message::DownloadBackup(backup_id) => {
                let config = self.config.clone();
                return Command::perform(
                    request_download_link(config, backup_id),
                    |result| match result {
                        Ok(url) => Message::OpenUrl(url),
                        Err(e) => Message::StatusChanged(format!("Error: {}", e)),
                    },
                );
            }
            Message::OpenUrl(url) => {
                if webbrowser::open(&url).is_err() {
                    self.status = "Failed to open web browser".to_string();
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        match self.view_state {
            ViewState::Main => column![
                text(&self.status),
                button("Setup").on_press(Message::Setup),
                button("View Logs").on_press(Message::ViewLogs),
                button("View Backups").on_press(Message::ViewBackups),
                button("Close Window").on_press(Message::Quit),
            ]
            .padding(20)
            .spacing(10)
            .into(),
            ViewState::Logs => {
                let header = row![]
                    .push(text("Timestamp").width(Length::Fixed(250.0)))
                    .push(text("Level").width(Length::Fixed(80.0)))
                    .push(text("Module").width(Length::Fixed(200.0)))
                    .push(text("Message").width(Length::Fill))
                    .spacing(10);

                let log_rows = self
                    .logs
                    .iter()
                    .enumerate()
                    .fold(column![].spacing(5), |col, (i, entry)| {
                        let style = if i % 2 == 0 {
                            iced::theme::Container::Custom(Box::new(styling::ContainerTheme::Even))
                        } else {
                            iced::theme::Container::Custom(Box::new(styling::ContainerTheme::Odd))
                        };

                        col.push(
                            container(
                                row![]
                                    .push(text(&entry.timestamp).width(Length::Fixed(250.0)))
                                    .push(text(&entry.level).width(Length::Fixed(80.0)))
                                    .push(text(&entry.module).width(Length::Fixed(200.0)))
                                    .push(text(&entry.message).width(Length::Fill))
                                    .spacing(10),
                            )
                            .style(style),
                        )
                    });

                let title_row = row![
                    text("Logs").size(24),
                    row![]
                        .width(Length::Fill)
                        .align_items(Alignment::End)
                        .spacing(10)
                        .push(button("Back").on_press(Message::BackToMain))
                ]
                .align_items(Alignment::Center)
                .spacing(20);

                column![title_row, header, scrollable(log_rows)]
                .padding(20)
                .spacing(10)
                .into()
            }
            ViewState::Backups => {
                let header = row![]
                    .push(text("ID").width(Length::FillPortion(1)))
                    .push(text("DB Name").width(Length::FillPortion(4)))
                    .push(text("Status").width(Length::FillPortion(2)))
                    .push(text("Completed At").width(Length::FillPortion(4)))
                    .push(
                        container(text("Download"))
                            .width(Length::FillPortion(2))
                            .center_x(),
                    )
                    .spacing(10)
                    .align_items(Alignment::Center);

                let backup_rows = self
                    .backups
                    .iter()
                    .enumerate()
                    .fold(column![].spacing(5), |col, (i, entry)| {
                        let style = if i % 2 == 0 {
                            iced::theme::Container::Custom(Box::new(styling::ContainerTheme::Even))
                        } else {
                            iced::theme::Container::Custom(Box::new(styling::ContainerTheme::Odd))
                        };

                        col.push(
                            container(
                                row![]
                                    .push(text(entry.id.to_string()).width(Length::FillPortion(1)))
                                    .push(text(&entry.db_name).width(Length::FillPortion(4)))
                                    .push(text(&entry.status).width(Length::FillPortion(2)))
                                    .push(
                                        text(&entry.backup_completed_at)
                                            .width(Length::FillPortion(4)),
                                    )
                                    .push(
                                        container(
                                            button("Download")
                                                .on_press(Message::DownloadBackup(entry.id)),
                                        )
                                        .width(Length::FillPortion(2))
                                        .center_x(),
                                    )
                                    .spacing(10)
                                    .align_items(Alignment::Center),
                            )
                            .style(style),
                        )
                    });

                let title_row = row![
                    text("Backups").size(24),
                    row![]
                        .width(Length::Fill)
                        .align_items(Alignment::End)
                        .spacing(10)
                        .push(button("Back").on_press(Message::BackToMain))
                ]
                .align_items(Alignment::Center)
                .spacing(20);

                column![title_row, header, scrollable(backup_rows)]
                    .padding(20)
                    .spacing(10)
                    .into()
            }
            ViewState::Settings => {
                let mut content = column![
                    text("Settings").size(24),
                    row![
                        text("Host:").width(Length::Fixed(120.0)),
                        text_input("", self.config.mssql.host.as_deref().unwrap_or(""))
                            .on_input(|s| Message::Config(ConfigMessage::HostChanged(s)))
                    ]
                    .spacing(5),
                    row![
                        text("Port:").width(Length::Fixed(120.0)),
                        text_input(
                            "",
                            &self.config.mssql.port.map(|p| p.to_string()).unwrap_or_default()
                        )
                        .on_input(|s| Message::Config(ConfigMessage::PortChanged(s)))
                    ]
                    .spacing(5),
                    row![
                        text("User:").width(Length::Fixed(120.0)),
                        text_input("", self.config.mssql.user.as_deref().unwrap_or(""))
                            .on_input(|s| Message::Config(ConfigMessage::UserChanged(s)))
                    ]
                    .spacing(5),
                    row![
                        text("Password:").width(Length::Fixed(120.0)),
                        text_input("", self.config.mssql.pass.as_deref().unwrap_or(""))
                            .on_input(|s| Message::Config(ConfigMessage::PassChanged(s)))
                    ]
                    .spacing(5),
                    row![
                        text("Database:").width(Length::Fixed(120.0)),
                        text_input("", &self.config.mssql.database)
                            .on_input(|s| Message::Config(ConfigMessage::DatabaseChanged(s)))
                    ]
                    .spacing(5),
                    row![
                        text("Instance Name:").width(Length::Fixed(120.0)),
                        text_input(
                            "",
                            self.config.mssql.instance_name.as_deref().unwrap_or("")
                        )
                        .on_input(|s| Message::Config(ConfigMessage::InstanceNameChanged(s)))
                    ]
                    .spacing(5),
                    row![
                        text("API URL:").width(Length::Fixed(120.0)),
                        text_input("", &self.config.api.url)
                            .on_input(|s| Message::Config(ConfigMessage::ApiUrlChanged(s)))
                    ]
                    .spacing(5),
                    row![
                        text("Server Token:").width(Length::Fixed(120.0)),
                        text_input("", &self.config.api.server_token)
                            .on_input(|s| Message::Config(ConfigMessage::ServerTokenChanged(s)))
                    ]
                    .spacing(5),
                    row![
                        text("Auth Token:").width(Length::Fixed(120.0)),
                        text_input("", &self.config.api.auth_token)
                            .on_input(|s| Message::Config(ConfigMessage::AuthTokenChanged(s)))
                    ]
                    .spacing(5),
                    row![
                        text("Temp Path:").width(Length::Fixed(120.0)),
                        text_input("", &self.config.backup.temp_path)
                            .on_input(|s| Message::Config(ConfigMessage::TempPathChanged(s)))
                    ]
                    .spacing(5),
                ]
                .spacing(10);

                let mut buttons = row![].spacing(10);
                if self.original_config.is_some() {
                    buttons = buttons.push(button("Cancel").on_press(Message::Cancel));
                }
                buttons = buttons.push(button("Save").on_press(Message::SaveConfig));

                content = content.push(buttons);

                content.padding(20).spacing(10).into()
            }
        }
    }
}

pub async fn run_app() -> Result<()> {
    let config = config::load_config("config.toml")?;
    let cleanup_config_path = config.backup.temp_path.clone();
    tokio::spawn(async move {
        cleanup::cleanup_task(cleanup_config_path).await;
    });
    loop {
        tracing::info!("Starting backup cycle...");
        match run_backup_cycle(&config).await {
            Ok(_) => tracing::info!("Backup cycle completed successfully."),
            Err(e) => tracing::error!("Backup cycle failed: {:?}", e),
        }
        tracing::info!("Waiting for 24 hours until the next cycle.");
        tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
    }
}

pub async fn run_backup_cycle(config: &config::Config) -> Result<()> {
    let start_time = OffsetDateTime::now_utc();
    let backup_filepath = match backup::perform_backup(config).await {
        Ok(path) => {
            tracing::info!("Backup created at: {:?}", path);
            path
        },
        Err(e) => anyhow::bail!("Failed to perform backup: {}", e),
    };
    if let Err(e) = backup::verify_backup(config, &backup_filepath).await {
        std::fs::remove_file(&backup_filepath)?;
        anyhow::bail!("Failed to verify backup: {}", e);
    }
    let end_time = OffsetDateTime::now_utc();
    let duration_seconds = (end_time - start_time).as_seconds_f64() as i64;
    let meta = upload::BackupMeta {
        start_time,
        end_time,
        duration_seconds,
        filepath: backup_filepath.clone(),
    };
    if let Err(e) = upload::upload_backup(config, meta).await {
        anyhow::bail!("Failed to upload backup: {}", e);
    }
    if let Err(e) = std::fs::remove_file(&backup_filepath) {
        tracing::error!("Failed to delete local backup file {:?}: {}", backup_filepath, e);
    } else {
        tracing::info!("Local backup file {:?} deleted.", backup_filepath);
    }
    Ok(())
}

fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let log_path = logging::get_log_filepath();
    let log_dir = log_path.parent().unwrap_or_else(|| Path::new("."));
    let log_filename = log_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("service.log"));

    let file_appender = tracing_appender::rolling::never(log_dir, log_filename);
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    let console_layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(std::io::stdout);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false);

    let filter = EnvFilter::new("mssql_backup_rust_service=info,iced=off");

    tracing_subscriber::registry()
        .with(filter)
        .with(console_layer)
        .with(file_layer)
        .init();

    guard
}

async fn fetch_backups(config: config::Config) -> Result<Vec<BackupEntry>, String> {
    let client = reqwest::Client::new();
    let url = format!("{}/api/backups", config.api.url.trim_end_matches('/'));
    let response = client
        .get(&url)
        .bearer_auth(&config.api.auth_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let backups = response
            .json::<Vec<BackupEntry>>()
            .await
            .map_err(|e| e.to_string())?;
        Ok(backups)
    } else {
        Err(format!("Failed to fetch backups: {}", response.status()))
    }
}

#[derive(serde::Deserialize)]
struct DownloadUrl {
    url: String,
}

async fn request_download_link(config: config::Config, backup_id: u64) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/backups/{}/download",
        config.api.url.trim_end_matches('/'),
        backup_id
    );
    let response = client
        .get(&url)
        .bearer_auth(&config.api.auth_token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        let download_url = response
            .json::<DownloadUrl>()
            .await
            .map_err(|e| e.to_string())?;
        Ok(download_url.url)
    } else {
        Err(format!(
            "Failed to request download link: {}",
            response.status()
        ))
    }
}
