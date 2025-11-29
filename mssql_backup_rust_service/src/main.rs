// #![windows_subsystem = "windows"]

mod config;
mod backup;
mod upload;
mod cleanup;
mod logging;

use makepad_widgets::*;
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

app_main!(App);

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

fn main() {
    // This is a dummy main function to satisfy the compiler.
    // The actual entry point is the `app_main!` macro.
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

async fn run_app() -> Result<()> {
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

async fn run_backup_cycle(config: &config::Config) -> Result<()> {
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
