//! Tracing initialization compatible with the legacy `dst-admin-go.log` file.
//!
//! The Go backend writes standard logs to both stdout and `./dst-admin-go.log`.
//! Rust startup uses `tracing` but keeps the same append-file behavior so
//! existing deployment scripts and log download routes can keep the file path.

use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use thiserror::Error;
use tracing_subscriber::fmt::MakeWriter;

/// Legacy default log filename used by the Go backend.
pub const DEFAULT_LOG_FILE: &str = "dst-admin-go.log";

/// Errors returned while installing the global tracing subscriber.
#[derive(Debug, Error)]
pub enum LoggingError {
    /// The requested log file could not be opened.
    #[error("failed to open log file `{}`: {source}", path.display())]
    OpenFile {
        /// Path that failed to open.
        path: PathBuf,
        /// Underlying filesystem error.
        #[source]
        source: io::Error,
    },
    /// A tracing subscriber was already installed or could not be installed.
    #[error("failed to install tracing subscriber: {0}")]
    Install(#[from] tracing::subscriber::SetGlobalDefaultError),
}

/// Initializes process-wide tracing to stdout and the provided log file path.
pub fn init(log_path: impl AsRef<Path>) -> Result<(), LoggingError> {
    let log_path = log_path.as_ref();
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .map_err(|source| LoggingError::OpenFile {
            path: log_path.to_path_buf(),
            source,
        })?;

    let subscriber = tracing_subscriber::fmt()
        .with_writer(StdoutAndFile::new(file))
        // The same formatted bytes go to stdout and the legacy log file.
        // Disable ANSI globally so downloaded `dst-admin-go.log` remains plain text.
        .with_ansi(false)
        .with_file(true)
        .with_line_number(true)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;
    tracing::info!(log_path = %log_path.display(), "initialized logging");

    Ok(())
}

#[derive(Clone)]
struct StdoutAndFile {
    file: Arc<Mutex<File>>,
}

impl StdoutAndFile {
    fn new(file: File) -> Self {
        Self {
            file: Arc::new(Mutex::new(file)),
        }
    }
}

impl<'writer> MakeWriter<'writer> for StdoutAndFile {
    type Writer = StdoutAndFileWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        StdoutAndFileWriter {
            file: Arc::clone(&self.file),
        }
    }
}

struct StdoutAndFileWriter {
    file: Arc<Mutex<File>>,
}

impl Write for StdoutAndFileWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        {
            let mut stdout = io::stdout().lock();
            stdout.write_all(buf)?;
            stdout.flush()?;
        }

        let mut file = self
            .file
            .lock()
            .map_err(|_| io::Error::other("log file lock poisoned"))?;
        file.write_all(buf)?;
        file.flush()?;

        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().lock().flush()?;
        self.file
            .lock()
            .map_err(|_| io::Error::other("log file lock poisoned"))?
            .flush()
    }
}
