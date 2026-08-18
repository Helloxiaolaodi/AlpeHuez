use std::time::Duration;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, ProcessRefreshKind, RefreshKind, System};
use tauri::Emitter;

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProcessInfo {
    pub name: String,
    pub cpu: f32,
    pub mem_mb: u64,
}

#[derive(serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct VelometerData {
    pub cpu: f32,
    pub mem_used: u64,
    pub mem_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub processes: Vec<ProcessInfo>,
}

pub fn spawn(app: tauri::AppHandle) {
    std::thread::spawn(move || {
        let mut sys = System::new_with_specifics(
            RefreshKind::new()
                .with_cpu(CpuRefreshKind::new().with_cpu_usage())
                .with_memory(MemoryRefreshKind::new().with_ram())
                .with_processes(ProcessRefreshKind::new().with_cpu().with_memory()),
        );
        loop {
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            sys.refresh_processes();
            let disks = sysinfo::Disks::new_with_refreshed_list();
            let disk = disks
                .iter()
                .find(|d| !d.is_removable())
                .or_else(|| disks.iter().next());
            let (disk_used, disk_total) = match disk {
                Some(d) => {
                    let total = d.total_space();
                    let avail = d.available_space();
                    (total.saturating_sub(avail), total)
                }
                None => (0, 0),
            };
            let mut processes: Vec<ProcessInfo> = sys
                .processes()
                .iter()
                .map(|(_, p)| ProcessInfo {
                    name: p.name().to_string(),
                    cpu: p.cpu_usage(),
                    mem_mb: p.memory() / (1024 * 1024),
                })
                .collect();
            processes.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
            processes.truncate(5);
            let data = VelometerData {
                cpu: sys.global_cpu_info().cpu_usage(),
                mem_used: sys.used_memory(),
                mem_total: sys.total_memory(),
                disk_used,
                disk_total,
                processes,
            };
            let _ = app.emit("velometer-update", data);
            std::thread::sleep(Duration::from_millis(1500));
        }
    });
}
