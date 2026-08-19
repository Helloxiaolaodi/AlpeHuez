use std::time::{Duration, Instant};
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, Networks, ProcessRefreshKind, RefreshKind, System};
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
    pub cpu_freq_mhz: u64,
    pub mem_used: u64,
    pub mem_total: u64,
    pub disk_used: u64,
    pub disk_total: u64,
    pub net_up_bps: u64,
    pub net_down_bps: u64,
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
        let mut networks = Networks::new_with_refreshed_list();
        // sysinfo 的进程 CPU 是单核百分比（多线程可 >100%），除以逻辑核心数对齐任务管理器。
        let logical_cores = sys.cpus().len().max(1) as f32;
        let mut last_tick = Instant::now();
        loop {
            let tick_start = Instant::now();
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            sys.refresh_processes();
            networks.refresh();
            // 网络上下行速率：refresh() 后 received()/transmitted() 返回自上次刷新以来的增量字节，
            // 除以实际间隔得到 bps。
            let now = Instant::now();
            let dt = now.duration_since(last_tick).as_secs_f32().max(0.001);
            last_tick = now;
            let (mut net_up, mut net_down) = (0u64, 0u64);
            for (_, n) in &networks {
                net_up += n.transmitted();
                net_down += n.received();
            }
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
                    cpu: p.cpu_usage() / logical_cores,
                    mem_mb: p.memory() / (1024 * 1024),
                })
                .collect();
            processes.sort_by(|a, b| b.cpu.partial_cmp(&a.cpu).unwrap_or(std::cmp::Ordering::Equal));
            processes.truncate(5);
            let data = VelometerData {
                cpu: sys.global_cpu_info().cpu_usage(),
                cpu_freq_mhz: sys.global_cpu_info().frequency(),
                mem_used: sys.used_memory(),
                mem_total: sys.total_memory(),
                disk_used,
                disk_total,
                net_up_bps: (net_up as f32 / dt) as u64,
                net_down_bps: (net_down as f32 / dt) as u64,
                processes,
            };
            let _ = app.emit("velometer-update", data);
            let elapsed = tick_start.elapsed();
            std::thread::sleep(Duration::from_millis(1500).saturating_sub(elapsed));
        }
    });
}
