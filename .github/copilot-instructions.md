# Orka Backup Codebase Guide for AI Agents

## Architecture Overview

This project implements a distributed MSSQL backup system with two main components:

1. **Rust Backup Service** (`mssql_backup_rust_service/`): A standalone Windows/Linux service that monitors MSSQL databases, creates backups, and uploads them to the API
2. **Laravel Admin API** (`mssql-backup-api/`): RESTful API + Filament admin panel for managing backups, servers, and users

### Data Flow
```
MSSQL Database → Backup Service (BACKUP DATABASE command)
    ↓ SHA256 checksum & verify
    ↓ Multipart form upload with metadata
    ↓ Laravel API validates server token + Sanctum auth
    ↓ Store .bak file + metadata to database
    ↓ Filament UI displays backup history & storage stats
```

## Critical Implementation Patterns

### Rust Backup Service (`mssql_backup_rust_service/src/`)

**Module responsibilities:**
- **`main.rs`**: Initializes logging via `tracing_appender` (file + console), spawns cleanup task, launches app
- **`app.rs`**: Makepad GUI (Setup/View Logs/Quit buttons) with async Tokio runtime inside Startup event
- **`backup.rs`**: Creates MSSQL connections (Tiberius library), executes `BACKUP DATABASE` + `RESTORE VERIFYONLY`
  - Platform-specific auth: Integrated auth (Windows) via `winauth`, Kerberos (Unix) via `gssapi`, or SQL auth
  - Encryption: `EncryptionLevel::Required`, `trust_cert()` for dev (see line 67)
- **`upload.rs`**: Multipart form submission with exponential backoff (1s → max 2^10s) over 10 attempts
  - Computes SHA256 checksum before upload
  - Sends: `token`, `database_name`, `backup_started_at`/`backup_completed_at` (RFC3339 format), `checksum_sha256`, binary file
  - Expects JSON response: `{"status": "ok", ...}` to confirm success
- **`cleanup.rs`**: Background task runs every 6 hours, deletes backup files older than 24h from temp path
- **`logging.rs`**: Log files stored in executable's directory with daily rotation (e.g., `service.log.2025-11-29`)
- **`config.rs`**: Loads TOML config from `config.toml` in working directory

**Key pattern:** All MSSQL operations use `async/await` with Tokio, errors propagate via `anyhow::Result`

### Laravel API (`mssql-backup-api/`)

**Database models:**
- `Server`: Has `token` (used by backup service), `group_id`, relationships to backups
- `Backup`: Stores metadata (checksum, duration, start/end times), has `user_id` (API caller) and `server_id`
- `User`: Belongs to `Group`, has Filament/Sanctum integration

**Key endpoints:**
- `POST /api/backups/upload` (BackupUploadController): Validates server token + checks `backup_file` multipart, stores file in `storage/app/backups/{server}/{database}/` with naming: `{timestamp}_{original_name}.bak`

**Filament admin features:**
- Resources: `BackupResource`, `ServerResource`, `UserResource`, `GroupResource`
- Widgets: `BackupChart`, `LatestBackups`, `StorageStats`

**Key pattern:** Sanctum token auth (Bearer token in header) + server token validation (form field)

## Build & Run Commands

### Rust Service
```bash
cd mssql_backup_rust_service
cargo build --release          # Produces binary in target/release/
cargo run                      # Debug mode with logging to console + file
cargo test                     # Unit tests (uses #[ctor] for logging setup)
```

### Laravel API
```bash
cd mssql-backup-api
composer setup                 # Full setup: install, .env, key-generate, migrate, npm install, build
composer dev                   # Run all services: artisan serve + queue:listen + pail + vite (concurrently)
composer test                  # Run PHPUnit tests
```

## Configuration

**Rust service** (`config.toml`):
- `[mssql]`: Database credentials (user/pass OR integrated auth), host, port, database name
- `[api]`: Backend API URL, `server_token` (registered in Laravel), `auth_token` (Sanctum API token)
- `[backup]`: `temp_path` for staging .bak files before upload (deleted by cleanup after 24h)

**Laravel API** (`.env`):
- `DATABASE_URL`: SQLite/MySQL connection (migrations auto-create tables)
- `SANCTUM_STATEFUL_DOMAINS`: For API token auth
- `FILESYSTEMS_DISKS_LOCAL_ROOT`: Storage path (default: `storage/app/`)

## Integration Points & Common Workflows

### Adding a New Backup Database
1. In Filament UI: Create `Server` with unique token and host
2. In Rust service: Add corresponding database to `config.toml` with server's token
3. Upload endpoint validates: server token exists → file stored → metadata logged

### Debugging Upload Failures
- Rust service logs to `service.log.{date}` file (check `logging.rs` for path logic)
- Exponential backoff: If API returns non-200 or invalid JSON, service retries with 2x delay
- Laravel: Check `storage/logs/` for API errors; Sanctum token may be expired

### Backup Verification Flow
1. `BACKUP DATABASE` command executed
2. `RESTORE VERIFYONLY` confirms integrity before upload
3. SHA256 computed during upload stream
4. API stores file + metadata together (atomicity via database transaction)

## Project-Specific Patterns to Follow

- **Error handling**: Use `anyhow::Result` + `?` operator in Rust; Laravel's exception handling for API
- **Logging**: Rust uses `tracing::` macros (info!/error!/debug!); configure in `main.rs` init
- **Time handling**: Rust uses `time` crate with `Rfc3339` for JSON serialization (RFC3339 format required for API)
- **Async operations**: All file I/O and network in Rust is `async` via Tokio; spawned on runtime
- **File management**: Backup files are temporary; automatic cleanup ensures disk doesn't bloat
- **Platform support**: Conditional compilation for Windows (winauth) vs Unix (Kerberos) in `Cargo.toml`

## Testing Locally

1. Ensure MSSQL instance is running (local or remote)
2. Create a test database and note credentials
3. Register a test server in Laravel (generates token)
4. Update `config.toml` with credentials and API endpoint
5. Run Rust service: `cargo run` — watch console + `service.log.{date}` for trace
6. Check Laravel: Backups appear in Filament UI after successful upload

## External Dependencies

- **Rust**: `tiberius` (MSSQL driver), `tokio` (async runtime), `tracing` (logging), `reqwest` (HTTP client), `sha2` (checksums)
- **PHP**: `Laravel 12`, `Filament 3` (admin UI), `Sanctum` (API tokens), `Vite` (asset bundling)
- **CLI tools**: `concurrently` (run multiple processes in dev), `npm`/`composer` (package managers)

