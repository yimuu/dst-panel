//! Helpers for Go-compatible log snapshot routes.
//!
//! The Go helper reads log files from the end and returns the newest lines
//! first. Keeping that order in a shared helper lets snapshot endpoints and
//! future stream tests use one compatibility implementation.

use std::{
    fmt,
    fs::File,
    io::{self, Read, Seek, SeekFrom},
};

/// Default number of log lines returned when the query omits `lines`.
pub const DEFAULT_LINE_LIMIT: usize = 100;

/// Maximum text bytes returned by one snapshot response.
///
/// Go's implementation can keep walking back through an arbitrarily large log.
/// Rust keeps line boundaries compatible but stops before one HTTP request can
/// allocate unbounded response text.
const MAX_LOG_SNAPSHOT_BYTES: usize = 8 * 1024 * 1024;
const MAX_LOG_SNAPSHOT_LINES: usize = 100_000;
const REVERSE_READ_CHUNK_SIZE: usize = 64 * 1024;

/// Error returned while reading a bounded log snapshot.
#[derive(Debug)]
pub enum RecentLinesError {
    Io(io::Error),
    SnapshotTooLarge,
}

impl fmt::Display for RecentLinesError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::SnapshotTooLarge => formatter.write_str("log snapshot is too large"),
        }
    }
}

impl std::error::Error for RecentLinesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::SnapshotTooLarge => None,
        }
    }
}

impl From<io::Error> for RecentLinesError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Parses Go's loose `lines` query behavior.
///
/// Gin's `DefaultQuery("lines", "100")` plus `strconv.Atoi` means an omitted
/// value uses 100, while malformed, zero, or negative values become a huge
/// unsigned count after Go casts `int` to `uint`. Rust represents that as
/// `usize::MAX`, which effectively returns all available lines.
pub fn parse_line_limit(value: Option<&str>) -> usize {
    let Some(value) = value else {
        return DEFAULT_LINE_LIMIT;
    };
    match value.parse::<i64>() {
        Ok(limit) if limit > 0 => limit as usize,
        _ => usize::MAX,
    }
}

/// Reads a bounded tail window and returns newest lines first.
///
/// Go's `ReverseRead` splits only on `\n`, so a newline-terminated file starts
/// with an empty string in the response and CRLF input keeps the `\r` byte in
/// each returned line. This helper keeps those compatibility details while
/// capping memory use to [`MAX_LOG_SNAPSHOT_BYTES`].
pub fn recent_lines_from_file(
    file: &mut File,
    limit: usize,
) -> Result<Vec<String>, RecentLinesError> {
    if limit == 0 {
        return Ok(Vec::new());
    }

    let file_size = file.metadata()?.len();
    let mut cursor = file_size;
    let mut current_line_reversed = Vec::new();
    let mut lines = Vec::new();
    let mut output_bytes = 0usize;

    while cursor > 0 && lines.len() < limit {
        let read_len = (cursor as usize).min(REVERSE_READ_CHUNK_SIZE);
        cursor -= read_len as u64;
        file.seek(SeekFrom::Start(cursor))?;

        let mut chunk = vec![0_u8; read_len];
        file.read_exact(&mut chunk)?;
        for byte in chunk.into_iter().rev() {
            if byte == b'\n' {
                push_reversed_line(
                    &mut lines,
                    &mut current_line_reversed,
                    &mut output_bytes,
                    limit,
                )?;
                if lines.len() == limit {
                    return Ok(lines);
                }
            } else {
                current_line_reversed.push(byte);
                output_bytes = output_bytes
                    .checked_add(1)
                    .ok_or(RecentLinesError::SnapshotTooLarge)?;
                ensure_snapshot_within_limits(lines.len(), output_bytes)?;
            }
        }
    }

    if lines.len() < limit {
        push_reversed_line(
            &mut lines,
            &mut current_line_reversed,
            &mut output_bytes,
            limit,
        )?;
    }
    Ok(lines)
}

#[cfg(test)]
fn recent_lines_from_tail_bytes(contents: &[u8], limit: usize) -> Vec<String> {
    contents
        .split(|byte| *byte == b'\n')
        .rev()
        .take(limit)
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect()
}

fn push_reversed_line(
    lines: &mut Vec<String>,
    current_line_reversed: &mut Vec<u8>,
    output_bytes: &mut usize,
    limit: usize,
) -> Result<(), RecentLinesError> {
    if lines.len() >= limit {
        return Ok(());
    }
    ensure_snapshot_within_limits(lines.len() + 1, *output_bytes)?;
    let line_bytes = current_line_reversed
        .iter()
        .rev()
        .copied()
        .collect::<Vec<_>>();
    lines.push(String::from_utf8_lossy(&line_bytes).into_owned());
    current_line_reversed.clear();
    Ok(())
}

fn ensure_snapshot_within_limits(
    line_count: usize,
    byte_count: usize,
) -> Result<(), RecentLinesError> {
    if line_count > MAX_LOG_SNAPSHOT_LINES || byte_count > MAX_LOG_SNAPSHOT_BYTES {
        Err(RecentLinesError::SnapshotTooLarge)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tempfile::tempfile;

    use super::{
        DEFAULT_LINE_LIMIT, parse_line_limit, recent_lines_from_file, recent_lines_from_tail_bytes,
    };

    #[test]
    fn parses_go_compatible_line_limits() {
        assert_eq!(parse_line_limit(None), DEFAULT_LINE_LIMIT);
        assert_eq!(parse_line_limit(Some("2")), 2);
        assert_eq!(parse_line_limit(Some("0")), usize::MAX);
        assert_eq!(parse_line_limit(Some("-1")), usize::MAX);
        assert_eq!(parse_line_limit(Some("not-a-number")), usize::MAX);
    }

    #[test]
    fn recent_lines_returns_newest_first_with_go_newline_shape() {
        assert_eq!(
            recent_lines_from_tail_bytes(b"one\r\ntwo\r\nthree\r\n", 3),
            vec!["".to_owned(), "three\r".to_owned(), "two\r".to_owned()]
        );
    }

    #[test]
    fn recent_lines_reads_from_file_tail() {
        let mut file = tempfile().unwrap();
        file.write_all(b"one\ntwo\nthree\nfour").unwrap();
        file.flush().unwrap();

        assert_eq!(
            recent_lines_from_file(&mut file, 2).unwrap(),
            vec!["four".to_owned(), "three".to_owned()]
        );
    }

    #[test]
    fn empty_file_matches_go_reverse_read_shape() {
        let mut file = tempfile().unwrap();

        assert_eq!(
            recent_lines_from_file(&mut file, 100).unwrap(),
            vec!["".to_owned()]
        );
    }
}
