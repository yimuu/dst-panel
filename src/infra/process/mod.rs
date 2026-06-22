//! Process snapshot helpers for migrated DST status and control routes.
//!
//! The Go backend queried process state with shell pipelines that interpolated
//! cluster and level names directly into the command string. This module keeps
//! the first Rust slice intentionally read-only and uses fixed command
//! arguments, so user-controlled names are matched in Rust instead of the
//! shell. Later start/stop/update routes should build on the same boundary.

use std::{io, process::Command};

#[cfg(any(test, windows))]
const MAX_POWERSHELL_CSV_BYTES: usize = 2 * 1024 * 1024;
#[cfg(any(test, windows))]
const MAX_POWERSHELL_CSV_FIELD_BYTES: usize = 16 * 1024;
#[cfg(any(test, windows))]
const MAX_POWERSHELL_CSV_RECORDS: usize = 1024;

/// A single OS process row with the fields used by the legacy dashboard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSnapshot {
    /// OS process id. It is optional so read-only status routes can still work
    /// on platforms or malformed rows where a pid is unavailable.
    pub pid: Option<u32>,
    /// Percent CPU column from `ps`, preserved as a string for Go JSON parity.
    pub cpu_usage: String,
    /// Percent memory column from `ps`, preserved as a string for Go JSON parity.
    pub mem_usage: String,
    /// Virtual memory size column from `ps`.
    pub virtual_size: String,
    /// Resident set size column from `ps`.
    pub resident_set_size: String,
    /// Full command line used only for literal Rust-side matching.
    pub command: String,
}

impl ProcessSnapshot {
    /// Returns true when a snapshot appears to belong to one DST shard.
    ///
    /// Matching is intentionally stricter than Go's
    /// `grep <cluster> | grep <level>` pipeline: the real Linux launcher starts
    /// `dontstarve_dedicated_server... -cluster <cluster> -shard <level>`.
    /// Requiring that shape avoids reporting `screen`, `tail`, or other helper
    /// processes as live shards while still preserving the public status fields.
    pub fn matches_level(&self, cluster_name: &str, level_name: &str) -> bool {
        let tokens = command_tokens(&self.command);
        if tokens
            .iter()
            .any(|token| helper_process_token(token.as_str()))
        {
            return false;
        }
        command_starts_like_dst_server(&tokens)
            && flag_value_equals(&tokens, "-cluster", cluster_name)
            && flag_value_equals(&tokens, "-shard", level_name)
    }
}

fn command_tokens(command: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut token = String::new();
    let mut in_quotes = false;

    for ch in command.chars() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ch if ch.is_whitespace() && !in_quotes => {
                if !token.is_empty() {
                    tokens.push(std::mem::take(&mut token));
                }
            }
            _ => token.push(ch),
        }
    }
    if !token.is_empty() {
        tokens.push(token);
    }

    tokens
}

fn helper_process_token(token: &str) -> bool {
    let leaf = path_leaf_token(token);
    leaf.eq_ignore_ascii_case("screen") || leaf.eq_ignore_ascii_case("tail")
}

fn command_starts_like_dst_server(tokens: &[String]) -> bool {
    let Some(first) = tokens.first() else {
        return false;
    };
    if is_dst_server_token(first) {
        return true;
    }
    if is_box_wrapper_token(first) {
        return tokens
            .get(1)
            .is_some_and(|token| is_dst_server_token(token));
    }
    false
}

fn is_dst_server_token(token: &str) -> bool {
    let leaf = path_leaf_token(token);
    matches!(
        leaf,
        "dontstarve_dedicated_server_nullrenderer"
            | "dontstarve_dedicated_server_nullrenderer_x64"
            | "dontstarve_dedicated_server_nullrenderer_x64_luajit"
            | "dontstarve_dedicated_server_nullrenderer.exe"
            | "dontstarve_dedicated_server_nullrenderer_x64.exe"
            | "dontstarve_dedicated_server_nullrenderer_x64_luajit.exe"
    )
}

fn is_box_wrapper_token(token: &str) -> bool {
    let leaf = path_leaf_token(token);
    leaf.eq_ignore_ascii_case("box86") || leaf.eq_ignore_ascii_case("box64")
}

fn path_leaf_token(token: &str) -> &str {
    token.rsplit(['/', '\\']).next().unwrap_or(token)
}

fn flag_value_equals(tokens: &[String], flag: &str, expected: &str) -> bool {
    tokens
        .windows(2)
        .any(|window| window[0] == flag && window[1] == expected)
}

/// Adapter trait used by tests and future command-control routes.
pub trait ProcessSnapshotProvider: Send + Sync {
    /// Returns current process snapshots without applying request-specific
    /// filtering. Implementations must not interpolate user input into shell.
    fn snapshots(&self) -> io::Result<Vec<ProcessSnapshot>>;
}

/// Real OS process snapshot provider.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemProcessSnapshotProvider;

impl ProcessSnapshotProvider for SystemProcessSnapshotProvider {
    fn snapshots(&self) -> io::Result<Vec<ProcessSnapshot>> {
        system_snapshots()
    }
}

/// Returns whether production process listing has a safe implementation.
///
/// Lifecycle routes use this provider as a write barrier before mutating DST
/// installs or cluster archives. Unsupported platforms must therefore fail
/// closed instead of reporting an empty list that looks like "nothing running".
pub fn system_snapshot_collection_supported() -> bool {
    cfg!(unix) || cfg!(windows)
}

/// Finds the first process matching a cluster and level.
pub fn first_level_process<'a>(
    snapshots: &'a [ProcessSnapshot],
    cluster_name: &str,
    level_name: &str,
) -> Option<&'a ProcessSnapshot> {
    snapshots
        .iter()
        .find(|snapshot| snapshot.matches_level(cluster_name, level_name))
}

#[cfg(unix)]
fn system_snapshots() -> io::Result<Vec<ProcessSnapshot>> {
    tracing::debug!(
        program = "ps",
        args = "-axo pid=,pcpu=,pmem=,vsz=,rss=,command=",
        "collecting process snapshots"
    );
    let output = Command::new("ps")
        .args(["-axo", "pid=,pcpu=,pmem=,vsz=,rss=,command="])
        .output()?;
    if !output.status.success() {
        tracing::warn!(
            status = ?output.status.code(),
            "ps command failed while collecting process snapshots"
        );
        return Err(io::Error::other("ps command failed"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(parse_ps_output(&stdout))
}

#[cfg(windows)]
fn system_snapshots() -> io::Result<Vec<ProcessSnapshot>> {
    use std::io::Read;

    let args = [
        "-NoProfile",
        "-Command",
        "Get-CimInstance Win32_Process | Select-Object ProcessId,CommandLine,WorkingSetSize | ConvertTo-Csv -NoTypeInformation",
    ];
    tracing::debug!(
        program = "powershell",
        args = ?args,
        "collecting process snapshots"
    );
    let mut child = Command::new("powershell")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let mut stdout = Vec::new();
    if let Some(stdout_pipe) = child.stdout.take() {
        let mut bounded_stdout = stdout_pipe.take((MAX_POWERSHELL_CSV_BYTES + 1) as u64);
        if let Err(error) = bounded_stdout.read_to_end(&mut stdout) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }
    }
    if stdout.len() > MAX_POWERSHELL_CSV_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        tracing::warn!(
            max_bytes = MAX_POWERSHELL_CSV_BYTES,
            "powershell process query output exceeded snapshot limit"
        );
        return Err(io::Error::other(
            "powershell process query output too large",
        ));
    }
    let status = child.wait()?;
    if !status.success() {
        tracing::warn!(
            status = ?status.code(),
            "powershell process query failed while collecting process snapshots"
        );
        return Err(io::Error::other("powershell process query failed"));
    }
    let stdout = String::from_utf8_lossy(&stdout);
    Ok(parse_powershell_process_csv(&stdout))
}

#[cfg(all(not(unix), not(windows)))]
fn system_snapshots() -> io::Result<Vec<ProcessSnapshot>> {
    tracing::warn!("process snapshot collection is not implemented for this platform yet");
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "process snapshot collection is unsupported on this platform",
    ))
}

#[cfg(unix)]
fn parse_ps_output(stdout: &str) -> Vec<ProcessSnapshot> {
    stdout
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid = parts.next()?.parse().ok();
            let cpu_usage = parts.next()?;
            let mem_usage = parts.next()?;
            let virtual_size = parts.next()?;
            let resident_set_size = parts.next()?;
            let command = parts.collect::<Vec<_>>().join(" ");
            if command.is_empty() {
                return None;
            }
            Some(ProcessSnapshot {
                pid,
                cpu_usage: cpu_usage.to_owned(),
                mem_usage: mem_usage.to_owned(),
                virtual_size: virtual_size.to_owned(),
                resident_set_size: resident_set_size.to_owned(),
                command,
            })
        })
        .collect()
}

#[cfg(any(test, windows))]
fn parse_powershell_process_csv(stdout: &str) -> Vec<ProcessSnapshot> {
    if stdout.len() > MAX_POWERSHELL_CSV_BYTES {
        tracing::warn!(
            bytes = stdout.len(),
            max_bytes = MAX_POWERSHELL_CSV_BYTES,
            "skipping oversized powershell process csv"
        );
        return Vec::new();
    }
    let mut records = parse_csv_records(stdout).into_iter();
    let Some(header) = records.next() else {
        return Vec::new();
    };
    let Some(pid_index) = header.iter().position(|field| field == "ProcessId") else {
        return Vec::new();
    };
    let Some(command_index) = header.iter().position(|field| field == "CommandLine") else {
        return Vec::new();
    };
    let Some(working_set_index) = header.iter().position(|field| field == "WorkingSetSize") else {
        return Vec::new();
    };

    records
        .filter_map(|record| {
            let command = record.get(command_index)?.to_owned();
            if command.trim().is_empty() {
                return None;
            }
            let resident_set_size = record
                .get(working_set_index)
                .and_then(|field| field.parse::<u64>().ok())
                .map(|bytes| (bytes / 1024).to_string())
                .unwrap_or_default();
            Some(ProcessSnapshot {
                pid: record.get(pid_index).and_then(|field| field.parse().ok()),
                cpu_usage: String::new(),
                mem_usage: String::new(),
                virtual_size: String::new(),
                resident_set_size,
                command,
            })
        })
        .collect()
}

#[cfg(any(test, windows))]
fn parse_csv_records(input: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut has_record_data = false;
    let mut record_invalid = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            match ch {
                '"' if chars.peek() == Some(&'"') => {
                    push_csv_field_char(&mut field, '"', &mut record, &mut record_invalid);
                    chars.next();
                }
                '"' => in_quotes = false,
                _ => push_csv_field_char(&mut field, ch, &mut record, &mut record_invalid),
            }
            has_record_data = true;
            continue;
        }

        match ch {
            '"' if field.is_empty() => {
                in_quotes = true;
                has_record_data = true;
            }
            ',' => {
                if !record_invalid {
                    record.push(std::mem::take(&mut field));
                }
                has_record_data = true;
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                finish_csv_record(
                    &mut records,
                    &mut record,
                    &mut field,
                    &mut has_record_data,
                    &mut record_invalid,
                );
            }
            '\n' => {
                finish_csv_record(
                    &mut records,
                    &mut record,
                    &mut field,
                    &mut has_record_data,
                    &mut record_invalid,
                );
            }
            _ => {
                push_csv_field_char(&mut field, ch, &mut record, &mut record_invalid);
                has_record_data = true;
            }
        }
    }

    finish_csv_record(
        &mut records,
        &mut record,
        &mut field,
        &mut has_record_data,
        &mut record_invalid,
    );
    records
}

#[cfg(any(test, windows))]
fn push_csv_field_char(
    field: &mut String,
    ch: char,
    record: &mut Vec<String>,
    record_invalid: &mut bool,
) {
    if *record_invalid {
        return;
    }
    if field.len().saturating_add(ch.len_utf8()) > MAX_POWERSHELL_CSV_FIELD_BYTES {
        field.clear();
        record.clear();
        *record_invalid = true;
        return;
    }
    field.push(ch);
}

#[cfg(any(test, windows))]
fn finish_csv_record(
    records: &mut Vec<Vec<String>>,
    record: &mut Vec<String>,
    field: &mut String,
    has_record_data: &mut bool,
    record_invalid: &mut bool,
) {
    if *record_invalid {
        record.clear();
        field.clear();
        *has_record_data = false;
        *record_invalid = false;
        return;
    }
    if !*has_record_data && record.is_empty() && field.is_empty() {
        return;
    }
    record.push(std::mem::take(field));
    if records.len() < MAX_POWERSHELL_CSV_RECORDS + 1 {
        records.push(std::mem::take(record));
    } else {
        record.clear();
    }
    *has_record_data = false;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn parses_ps_output_without_shell_specific_assumptions() {
        let snapshots = parse_ps_output(
            " 1234  0.1  1.2 123456 7890 /srv/dst/bin/dontstarve_dedicated_server_nullrenderer -cluster ClusterA -shard Master\n",
        );

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].pid, Some(1234));
        assert_eq!(snapshots[0].cpu_usage, "0.1");
        assert_eq!(snapshots[0].mem_usage, "1.2");
        assert_eq!(snapshots[0].virtual_size, "123456");
        assert_eq!(snapshots[0].resident_set_size, "7890");
        assert!(snapshots[0].matches_level("ClusterA", "Master"));
    }

    #[test]
    fn parses_powershell_csv_with_crlf_and_quoted_values() {
        let snapshots = parse_powershell_process_csv(
            "\"ProcessId\",\"CommandLine\",\"WorkingSetSize\"\r\n\"4321\",\"C:\\dst\\dontstarve_dedicated_server_nullrenderer_x64.exe -cluster ClusterWin -shard Master\",\"104857600\"\r\n",
        );

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].pid, Some(4321));
        assert_eq!(snapshots[0].resident_set_size, "102400");
        assert_eq!(snapshots[0].cpu_usage, "");
        assert_eq!(snapshots[0].mem_usage, "");
        assert_eq!(snapshots[0].virtual_size, "");
        assert!(snapshots[0].matches_level("ClusterWin", "Master"));
    }

    #[test]
    fn parses_powershell_csv_with_quoted_windows_executable_path() {
        let snapshots = parse_powershell_process_csv(
            "\"ProcessId\",\"CommandLine\",\"WorkingSetSize\"\r\n\"4321\",\"\"\"C:\\Program Files\\dst\\dontstarve_dedicated_server_nullrenderer_x64.exe\"\" -cluster ClusterWin -shard Master\",\"104857600\"\r\n",
        );

        assert_eq!(snapshots.len(), 1);
        assert!(snapshots[0].matches_level("ClusterWin", "Master"));
    }

    #[test]
    fn parses_powershell_csv_with_escaped_quotes_and_commas_in_command() {
        let snapshots = parse_powershell_process_csv(
            "\"CommandLine\",\"WorkingSetSize\",\"ProcessId\"\r\n\"C:\\dst\\dontstarve_dedicated_server_nullrenderer_x64.exe -cluster ClusterWin -shard Caves -note \"\"alpha,beta\"\"\",\"2048\",\"99\"\r\n",
        );

        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].pid, Some(99));
        assert_eq!(snapshots[0].resident_set_size, "2");
        assert_eq!(
            snapshots[0].command,
            "C:\\dst\\dontstarve_dedicated_server_nullrenderer_x64.exe -cluster ClusterWin -shard Caves -note \"alpha,beta\""
        );
        assert!(snapshots[0].matches_level("ClusterWin", "Caves"));
    }

    #[test]
    fn parse_powershell_csv_skips_empty_or_malformed_command_rows() {
        let snapshots = parse_powershell_process_csv(
            "\"ProcessId\",\"CommandLine\",\"WorkingSetSize\"\r\n\"1\",\"\",\"1024\"\r\n\"2\",\"   \",\"2048\"\r\n\"3\"\r\n",
        );

        assert!(snapshots.is_empty());
    }

    #[test]
    fn parse_powershell_csv_handles_trailing_empty_fields() {
        let records = parse_csv_records("\"a\",\"b\",\r\n\"1\",\"2\",\r\n");

        assert_eq!(records, vec![vec!["a", "b", ""], vec!["1", "2", ""]]);
    }

    #[test]
    fn parse_powershell_csv_does_not_panic_on_unbalanced_quotes() {
        let snapshots = parse_powershell_process_csv(
            "\"ProcessId\",\"CommandLine\",\"WorkingSetSize\"\r\n\"1\",\"C:\\dst\\dontstarve_dedicated_server_nullrenderer_x64.exe -cluster ClusterWin -shard Master,\"1024\"\r\n",
        );

        assert_eq!(snapshots.len(), 1);
        assert!(snapshots[0].command.contains("ClusterWin"));
    }

    #[test]
    fn parse_powershell_csv_rejects_oversized_fields() {
        let oversized_command = "x".repeat(MAX_POWERSHELL_CSV_FIELD_BYTES + 1);
        let csv = format!(
            "\"ProcessId\",\"CommandLine\",\"WorkingSetSize\"\r\n\"1\",\"{oversized_command}\",\"1024\"\r\n"
        );

        assert!(parse_powershell_process_csv(&csv).is_empty());
    }

    #[test]
    fn parse_powershell_csv_caps_record_count() {
        let mut csv = "\"ProcessId\",\"CommandLine\",\"WorkingSetSize\"\r\n".to_owned();
        for index in 0..(MAX_POWERSHELL_CSV_RECORDS + 5) {
            csv.push_str(&format!(
                "\"{index}\",\"C:\\dst\\dontstarve_dedicated_server_nullrenderer_x64.exe -cluster ClusterWin -shard Master\",\"1024\"\r\n"
            ));
        }

        assert_eq!(
            parse_powershell_process_csv(&csv).len(),
            MAX_POWERSHELL_CSV_RECORDS
        );
    }

    #[test]
    fn process_matching_requires_dst_cluster_and_shard_arguments() {
        let snapshots = vec![ProcessSnapshot {
            pid: Some(1234),
            cpu_usage: "0.1".to_owned(),
            mem_usage: "0.2".to_owned(),
            virtual_size: "10".to_owned(),
            resident_set_size: "20".to_owned(),
            command: "/srv/dst/bin/dontstarve_dedicated_server_nullrenderer -console -cluster ClusterSafe -shard Master"
                .to_owned(),
        }];

        assert!(first_level_process(&snapshots, "ClusterSafe", "Master").is_some());
        assert!(first_level_process(&snapshots, "Cluster", "Master").is_none());
        assert!(first_level_process(&snapshots, "ClusterSafe", "Mast").is_none());
    }

    #[test]
    fn process_matching_ignores_screen_and_tail_helpers() {
        let snapshots = vec![
            ProcessSnapshot {
                pid: Some(1111),
                cpu_usage: "9.9".to_owned(),
                mem_usage: "8.8".to_owned(),
                virtual_size: "777".to_owned(),
                resident_set_size: "666".to_owned(),
                command: "SCREEN -S dst-ClusterSafe-Master".to_owned(),
            },
            ProcessSnapshot {
                pid: Some(2222),
                cpu_usage: "7.7".to_owned(),
                mem_usage: "6.6".to_owned(),
                virtual_size: "555".to_owned(),
                resident_set_size: "444".to_owned(),
                command: "tail -f /var/log/ClusterSafe/Master/server_log.txt".to_owned(),
            },
            ProcessSnapshot {
                pid: Some(3333),
                cpu_usage: "0.1".to_owned(),
                mem_usage: "0.2".to_owned(),
                virtual_size: "10".to_owned(),
                resident_set_size: "20".to_owned(),
                command: "./dontstarve_dedicated_server_nullrenderer -console -cluster ClusterSafe -shard Master"
                    .to_owned(),
            },
        ];

        let process = first_level_process(&snapshots, "ClusterSafe", "Master").unwrap();

        assert_eq!(process.cpu_usage, "0.1");
    }

    #[test]
    fn process_matching_rejects_wrappers_that_only_quote_dst_arguments() {
        let snapshots = vec![ProcessSnapshot {
            pid: Some(1234),
            cpu_usage: "0.1".to_owned(),
            mem_usage: "0.2".to_owned(),
            virtual_size: "10".to_owned(),
            resident_set_size: "20".to_owned(),
            command: "monitor -- /srv/dst/bin/dontstarve_dedicated_server_nullrenderer -cluster ClusterSafe -shard Master"
                .to_owned(),
        }];

        assert!(first_level_process(&snapshots, "ClusterSafe", "Master").is_none());
    }

    #[test]
    fn process_matching_rejects_dst_named_helper_binaries() {
        let snapshots = vec![ProcessSnapshot {
            pid: Some(4321),
            cpu_usage: "0.1".to_owned(),
            mem_usage: "0.2".to_owned(),
            virtual_size: "10".to_owned(),
            resident_set_size: "20".to_owned(),
            command:
                "/tools/dontstarve_dedicated_server_watchdog -cluster ClusterSafe -shard Master"
                    .to_owned(),
        }];

        assert!(first_level_process(&snapshots, "ClusterSafe", "Master").is_none());
    }

    #[test]
    fn process_matching_keeps_dst_binary_name_case_sensitive() {
        let snapshots = vec![ProcessSnapshot {
            pid: Some(1234),
            cpu_usage: "0.1".to_owned(),
            mem_usage: "0.2".to_owned(),
            virtual_size: "10".to_owned(),
            resident_set_size: "20".to_owned(),
            command:
                "C:\\dst\\DONTSTARVE_DEDICATED_SERVER_NULLRENDERER_X64.EXE -cluster ClusterSafe -shard Master"
                    .to_owned(),
        }];

        assert!(first_level_process(&snapshots, "ClusterSafe", "Master").is_none());
    }

    #[test]
    fn system_snapshot_provider_fails_closed_when_process_listing_is_unsupported() {
        let result = SystemProcessSnapshotProvider.snapshots();

        if system_snapshot_collection_supported() {
            assert!(
                result.is_ok()
                    || result
                        .as_ref()
                        .is_err_and(|error| error.kind() != io::ErrorKind::Unsupported),
                "supported platforms may report real ps errors but not unsupported"
            );
        } else {
            let error =
                result.expect_err("unsupported platforms must not report an empty snapshot list");
            assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        }
    }
}
