//! Game status and dashboard DTOs used by migrated `/api/game/*` routes.
//!
//! This slice covers read-only status/system endpoints. Commanding the game
//! server remains outside this module until the start/stop/update routes are
//! migrated behind command adapters.

use std::{collections::BTreeMap, fs, path::Path};

use serde::Serialize;

use crate::{
    infra::process,
    validation::{ValidationError, validate_safe_command_arg},
};

pub(crate) mod console;
pub(crate) mod level;
pub(crate) mod lifecycle;
pub(crate) mod mod_setup;
pub(crate) mod player_query;
pub(crate) mod preinstall;
pub(crate) mod udp;

use level::World;

/// Practical guardrail for malformed `/proc/stat` rows such as
/// `cpu1000000000`, which must not drive sparse vector allocation.
const MAX_PROC_STAT_CPU_COUNT: usize = 4096;

/// Go `vo.DstPsVo` JSON shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct DstPsVo {
    /// Legacy field spelling from Go: `CpuUage`.
    #[serde(rename = "cpuUage")]
    cpu_usage: String,
    /// Legacy field spelling from Go: `MemUage`.
    #[serde(rename = "memUage")]
    mem_usage: String,
    /// Go keeps this field uppercase in JSON.
    #[serde(rename = "VSZ")]
    virtual_size: String,
    /// Go keeps this field uppercase in JSON.
    #[serde(rename = "RSS")]
    resident_set_size: String,
}

impl From<&process::ProcessSnapshot> for DstPsVo {
    fn from(snapshot: &process::ProcessSnapshot) -> Self {
        Self {
            cpu_usage: snapshot.cpu_usage.clone(),
            mem_usage: snapshot.mem_usage.clone(),
            virtual_size: snapshot.virtual_size.clone(),
            resident_set_size: snapshot.resident_set_size.clone(),
        }
    }
}

/// Go `api.LevelInfo` JSON shape returned by `/api/game/8level/status`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LevelStatusInfo {
    /// Legacy field is uppercase `Ps`, not `ps`.
    #[serde(rename = "Ps")]
    ps: DstPsVo,
    status: bool,
    #[serde(rename = "levelName")]
    level_name: String,
    #[serde(rename = "is_master")]
    is_master: bool,
    uuid: String,
    leveldataoverride: String,
    modoverrides: String,
    server_ini: crate::dst::server_ini::ServerIni,
}

/// Adds process state to file-backed level metadata.
pub(crate) fn level_statuses_from_snapshots(
    cluster_name: &str,
    worlds: Vec<World>,
    snapshots: &[process::ProcessSnapshot],
) -> Result<Vec<LevelStatusInfo>, ValidationError> {
    worlds
        .into_iter()
        .map(|world| {
            let level_arg = validate_safe_command_arg("level name", &world.uuid)?;
            let process = process::first_level_process(snapshots, cluster_name, level_arg.as_str());
            Ok(LevelStatusInfo {
                ps: process.map(DstPsVo::from).unwrap_or_default(),
                status: process.is_some(),
                level_name: world.level_name,
                is_master: world.is_master,
                uuid: world.uuid,
                leveldataoverride: world.leveldataoverride,
                modoverrides: world.modoverrides,
                server_ini: world.server_ini,
            })
        })
        .collect()
}

/// Go `service.SystemInfo` JSON shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SystemInfo {
    host: HostInfo,
    cpu: CpuInfo,
    mem: MemInfo,
    disk: DiskInfo,
    #[serde(rename = "panelMemUsage")]
    panel_mem_usage: u64,
    #[serde(rename = "panelCpuUsage")]
    panel_cpu_usage: f64,
}

/// Go `systemUtils.HostInfo` JSON shape.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HostInfo {
    os: String,
    hostname: String,
    platform: String,
    #[serde(rename = "kernelArch")]
    kernel_arch: String,
}

/// Go `systemUtils.CpuInfo` JSON shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CpuInfo {
    cores: i64,
    #[serde(rename = "cpuPercent")]
    cpu_percent: Vec<f64>,
    #[serde(rename = "cpuUsedPercent")]
    cpu_used_percent: f64,
    #[serde(rename = "cpuUsed")]
    cpu_used: f64,
}

/// Go `systemUtils.MemInfo` JSON shape.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub struct MemInfo {
    total: u64,
    available: u64,
    used: u64,
    #[serde(rename = "usedPercent")]
    used_percent: f64,
}

/// Go `systemUtils.DiskInfo` JSON shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiskInfo {
    devices: Vec<DeviceInfo>,
}

/// Go `deviceInfo` JSON shape.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DeviceInfo {
    device: String,
    mountpoint: String,
    fstype: String,
    opts: String,
    total: u64,
    usage: f64,
    #[serde(rename = "inodesUsage")]
    inodes_usage: f64,
}

/// Collects a best-effort dashboard snapshot without network access.
///
/// The Go implementation uses `gopsutil`; adding that dependency is deferred
/// for this migration slice. The Rust shape is compatible and values are
/// populated from stable standard-library or libc sources where available.
pub(crate) fn collect_system_info(root_path: &Path) -> SystemInfo {
    tracing::debug!("collecting game system info snapshot");
    SystemInfo {
        host: collect_host_info(),
        cpu: collect_cpu_info(),
        mem: collect_mem_info(),
        disk: collect_disk_info(root_path),
        panel_mem_usage: collect_panel_mem_usage(),
        panel_cpu_usage: 0.0,
    }
}

fn collect_host_info() -> HostInfo {
    HostInfo {
        os: std::env::consts::OS.to_owned(),
        hostname: hostname(),
        platform: collect_platform(),
        kernel_arch: std::env::consts::ARCH.to_owned(),
    }
}

fn collect_platform() -> String {
    #[cfg(target_os = "linux")]
    {
        match fs::read_to_string("/etc/os-release") {
            Ok(contents) => {
                let platform = platform_from_os_release(&contents);
                if platform.is_empty() {
                    tracing::debug!("/etc/os-release did not contain ID; falling back to OS name");
                    std::env::consts::OS.to_owned()
                } else {
                    platform
                }
            }
            Err(error) => {
                tracing::warn!(
                    %error,
                    "failed to read /etc/os-release while collecting host platform"
                );
                std::env::consts::OS.to_owned()
            }
        }
    }

    #[cfg(not(target_os = "linux"))]
    {
        std::env::consts::OS.to_owned()
    }
}

pub fn platform_from_os_release(contents: &str) -> String {
    contents
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            if key == "ID" {
                Some(unquote_os_release_value(value.trim()))
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn unquote_os_release_value(value: &str) -> String {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
        .to_owned()
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            fs::read_to_string("/etc/hostname")
                .ok()
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
}

fn collect_cpu_info() -> CpuInfo {
    let cores = std::thread::available_parallelism()
        .map(|count| i64::try_from(count.get()).unwrap_or(i64::MAX))
        .unwrap_or_default();

    #[cfg(target_os = "linux")]
    {
        let previous = match fs::read_to_string("/proc/stat") {
            Ok(contents) => contents,
            Err(error) => {
                tracing::warn!(%error, "failed to read /proc/stat before CPU sampling");
                return unavailable_cpu_info(cores);
            }
        };
        std::thread::sleep(std::time::Duration::from_millis(100));
        let current = match fs::read_to_string("/proc/stat") {
            Ok(contents) => contents,
            Err(error) => {
                tracing::warn!(%error, "failed to read /proc/stat after CPU sampling");
                return unavailable_cpu_info(cores);
            }
        };
        cpu_info_from_proc_stat_pair(&previous, &current, cores)
    }

    #[cfg(not(target_os = "linux"))]
    {
        unavailable_cpu_info(cores)
    }
}

fn unavailable_cpu_info(cores: i64) -> CpuInfo {
    CpuInfo {
        cores,
        cpu_percent: Vec::new(),
        cpu_used_percent: 0.0,
        cpu_used: 0.0,
    }
}

pub fn cpu_info_from_proc_stat_pair(previous: &str, current: &str, cores: i64) -> CpuInfo {
    let previous_rows = parse_proc_stat_cpu_rows(previous);
    let current_rows = parse_proc_stat_cpu_rows(current);
    let cpu_used_percent = previous_rows
        .aggregate
        .zip(current_rows.aggregate)
        .map_or(0.0, |(previous, current)| usage_between(previous, current));

    let core_count = current_rows
        .per_core
        .keys()
        .next_back()
        .map_or(0, |index| index + 1);
    let cpu_percent = (0..core_count)
        .map(|index| {
            previous_rows
                .per_core
                .get(&index)
                .zip(current_rows.per_core.get(&index))
                .map_or(0.0, |(previous, current)| {
                    usage_between(*previous, *current)
                })
        })
        .collect();
    let cpu_used = cpu_used_percent * 0.01 * cores.max(0) as f64;

    CpuInfo {
        cores,
        cpu_percent,
        cpu_used_percent,
        cpu_used,
    }
}

#[derive(Debug, Default)]
struct ProcStatCpuRows {
    aggregate: Option<CpuTimes>,
    per_core: BTreeMap<usize, CpuTimes>,
}

#[derive(Debug, Clone, Copy)]
struct CpuTimes {
    total: u64,
    idle: u64,
}

fn parse_proc_stat_cpu_rows(contents: &str) -> ProcStatCpuRows {
    let mut rows = ProcStatCpuRows::default();
    for line in contents.lines() {
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else {
            continue;
        };
        if name != "cpu" && !name.strip_prefix("cpu").is_some_and(cpu_name_is_core) {
            continue;
        }
        let Some(values) = parse_proc_stat_values(parts) else {
            tracing::debug!(line = %line, "skipping malformed /proc/stat CPU row");
            continue;
        };
        if values.len() < 4 {
            tracing::debug!(line = %line, "skipping malformed /proc/stat CPU row");
            continue;
        }
        let Some(total) = checked_sum(values.iter().copied()) else {
            tracing::debug!(line = %line, "skipping overflowing /proc/stat CPU row");
            continue;
        };
        let Some(idle) = values[3].checked_add(values.get(4).copied().unwrap_or_default()) else {
            tracing::debug!(line = %line, "skipping overflowing /proc/stat CPU idle values");
            continue;
        };
        let times = CpuTimes { total, idle };
        if name == "cpu" {
            rows.aggregate = Some(times);
        } else {
            let Some(index) = cpu_core_index(name) else {
                tracing::debug!(line = %line, "skipping /proc/stat CPU row with invalid core index");
                continue;
            };
            if index >= MAX_PROC_STAT_CPU_COUNT {
                tracing::debug!(
                    line = %line,
                    index,
                    max_cpu_count = MAX_PROC_STAT_CPU_COUNT,
                    "skipping /proc/stat CPU row with unreasonable core index"
                );
                continue;
            }
            rows.per_core.insert(index, times);
        }
    }
    rows
}

fn parse_proc_stat_values<'a>(values: impl Iterator<Item = &'a str>) -> Option<Vec<u64>> {
    values.map(|value| value.parse().ok()).collect()
}

fn checked_sum(mut values: impl Iterator<Item = u64>) -> Option<u64> {
    values.try_fold(0_u64, |total, value| total.checked_add(value))
}

fn cpu_core_index(name: &str) -> Option<usize> {
    name.strip_prefix("cpu")?.parse().ok()
}

fn cpu_name_is_core(suffix: &str) -> bool {
    !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
}

fn usage_between(previous: CpuTimes, current: CpuTimes) -> f64 {
    let delta_total = current.total.saturating_sub(previous.total);
    if delta_total == 0 {
        return 0.0;
    }
    let delta_idle = current.idle.saturating_sub(previous.idle);
    (delta_total.saturating_sub(delta_idle) as f64 / delta_total as f64) * 100.0
}

fn collect_mem_info() -> MemInfo {
    #[cfg(target_os = "linux")]
    {
        if let Some(info) = linux_mem_info() {
            return info;
        }
    }

    tracing::debug!("memory info is unavailable on this platform; returning zeroed values");
    MemInfo::default()
}

#[cfg(target_os = "linux")]
fn linux_mem_info() -> Option<MemInfo> {
    let contents = fs::read_to_string("/proc/meminfo").ok()?;
    let mut total = None;
    let mut available = None;
    for line in contents.lines() {
        let mut parts = line.split_whitespace();
        let key = parts.next()?;
        let value_kib: u64 = parts.next()?.parse().ok()?;
        match key {
            "MemTotal:" => total = Some(value_kib.saturating_mul(1024)),
            "MemAvailable:" => available = Some(value_kib.saturating_mul(1024)),
            _ => {}
        }
    }
    let total = total?;
    let available = available.unwrap_or_default();
    let used = total.saturating_sub(available);
    let used_percent = if total == 0 {
        0.0
    } else {
        (used as f64 / total as f64) * 100.0
    };
    Some(MemInfo {
        total,
        available,
        used,
        used_percent,
    })
}

fn collect_disk_info(root_path: &Path) -> DiskInfo {
    DiskInfo {
        devices: disk_devices(root_path),
    }
}

#[cfg(unix)]
fn disk_devices(root_path: &Path) -> Vec<DeviceInfo> {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};

    let Ok(path) = CString::new(root_path.as_os_str().as_bytes()) else {
        tracing::warn!("root path contains a nul byte; disk info unavailable");
        return Vec::new();
    };
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `path` is a valid nul-terminated C string and `stat` points to
    // writable memory for libc to initialize.
    let rc = unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) };
    if rc != 0 {
        tracing::warn!(
            error = %std::io::Error::last_os_error(),
            "statvfs failed while collecting disk info"
        );
        return Vec::new();
    }
    // SAFETY: `statvfs` returned success, so libc initialized the structure.
    let stat = unsafe { stat.assume_init() };
    let block_size = u128::from(stat.f_frsize.max(1));
    let total_bytes = u128::from(stat.f_blocks).saturating_mul(block_size);
    let free_bytes = u128::from(stat.f_bfree).saturating_mul(block_size);
    let used_bytes = total_bytes.saturating_sub(free_bytes);
    let usage = if total_bytes == 0 {
        0.0
    } else {
        (used_bytes as f64 / total_bytes as f64) * 100.0
    };
    let inodes_usage = if stat.f_files == 0 {
        0.0
    } else {
        let used_inodes = stat.f_files.saturating_sub(stat.f_ffree);
        (used_inodes as f64 / stat.f_files as f64) * 100.0
    };

    vec![DeviceInfo {
        device: String::new(),
        mountpoint: root_path.display().to_string(),
        fstype: String::new(),
        opts: String::new(),
        total: u64::try_from(total_bytes / 1024 / 1024).unwrap_or(u64::MAX),
        usage,
        inodes_usage,
    }]
}

#[cfg(not(unix))]
fn disk_devices(_root_path: &Path) -> Vec<DeviceInfo> {
    tracing::debug!("disk info is unavailable on this platform; returning no devices");
    Vec::new()
}

#[cfg(unix)]
fn collect_panel_mem_usage() -> u64 {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    // SAFETY: `usage` points to valid writable memory for libc to initialize.
    let rc = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if rc != 0 {
        tracing::warn!(
            error = %std::io::Error::last_os_error(),
            "getrusage failed while collecting panel memory usage"
        );
        return 0;
    }
    // SAFETY: `getrusage` returned success, so libc initialized the structure.
    let usage = unsafe { usage.assume_init() };
    u64::try_from(usage.ru_maxrss).unwrap_or_default()
}

#[cfg(not(unix))]
fn collect_panel_mem_usage() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use crate::dst::server_ini::ServerIni;

    use super::*;

    #[test]
    fn level_status_uses_literal_process_snapshot_matching() {
        let worlds = vec![World {
            level_name: "森林".to_owned(),
            is_master: true,
            uuid: "Master".to_owned(),
            leveldataoverride: "return {}".to_owned(),
            modoverrides: "return {}".to_owned(),
            server_ini: ServerIni::master_default(),
        }];
        let snapshots = vec![process::ProcessSnapshot {
            pid: Some(1234),
            cpu_usage: "1.0".to_owned(),
            mem_usage: "2.0".to_owned(),
            virtual_size: "123".to_owned(),
            resident_set_size: "45".to_owned(),
            command:
                "./dontstarve_dedicated_server_nullrenderer -cluster ClusterProc -shard Master"
                    .to_owned(),
        }];

        let statuses = level_statuses_from_snapshots("ClusterProc", worlds, &snapshots).unwrap();

        assert_eq!(statuses.len(), 1);
        assert!(statuses[0].status);
        assert_eq!(statuses[0].ps.cpu_usage, "1.0");
        assert_eq!(statuses[0].ps.mem_usage, "2.0");
    }
}
