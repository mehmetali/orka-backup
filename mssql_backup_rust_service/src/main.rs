// #![windows_subsystem = "windows"]

use iced::{Application, Command, Element, Settings, Theme};
use iced::widget::{button, column, text};

mod config;
mod backup;
mod upload;
mod cleanup;
mod logging;

use anyhow::Result;
use std::path::Path;
use std::time::Duration;
use ctor::ctor;
use time::OffsetDateTime;
use tracing_subscriber::prelude::*;
use once_cell::sync::Lazy;

// This static guard will be initialized once, ensuring the logging thread
// stays alive for the duration of the application.
static LOGGING_GUARD: Lazy<tracing_appender::non_blocking::WorkerGuard> = Lazy::new(init_logging);

#[ctor]
fn early_init() {
    // Accessing the Lazy guard will initialize it.
    Lazy::force(&LOGGING_GUARD);
    tracing::info!("Early init logging complete.");
}

pub fn main() -> iced::Result {
    App::run(Settings::default())
}

struct App {
    status: String,
}

#[derive(Debug, Clone)]
enum Message {
    Setup,
    ViewLogs,
    Quit,
    StatusChanged(String),
}

impl Application for App {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let (status, command) = if Path::new("config.toml").exists() {
            (
                "Backup service running...".to_string(),
                Command::perform(run_app_wrapper(), Message::StatusChanged),
            )
        } else {
            (
                "Config file not found. Please set up the application.".to_string(),
                Command::none(),
            )
        };

        (Self { status }, command)
    }

    fn title(&self) -> String {
        String::from("MSSQL Backup Service")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Setup => {
                println!("Setup button clicked!");
            }
            Message::ViewLogs => {
                println!("View Logs button clicked!");
            }
            Message::Quit => {
                return Command::perform(async {}, |_| std::process::exit(0));
            }
            Message::StatusChanged(new_status) => {
                self.status = new_status;
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        column![
            text(&self.status),
            button("Setup").on_press(Message::Setup),
            button("View Logs").on_press(Message::ViewLogs),
            button("Quit").on_press(Message::Quit),
        ]
        .padding(20)
        .spacing(10)
        .into()
    }
}

async fn run_app_wrapper() -> String {
    if let Err(e) = run_app().await {
        let msg = format!("Backup thread failed: {}", e);
        tracing::error!("{}", msg);
        return msg;
    }
    "Backup service finished.".to_string()
}


fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    let log_path = logging::get_log_filepath();
    let log_dir = log_path.parent().unwrap_or_else(|| Path::new("."));
    let log_filename = log_path.file_name().unwrap_or_else(|| std::ffi::OsStr::new("service.log"));

    let file_appender = tracing_appender::rolling::never(log_dir, log_filename);
    let (non_blocking_file, guard) = tracing_appender::non_blocking(file_appender);

    let console_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout);

    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking_file)
        .with_ansi(false);

    tracing_subscriber::registry()
        .with(console_layer)
        .with(file_layer)
        .init();

    guard
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
            Err(e) => tracing::error!("Backup cycle failed: {}", e),
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
