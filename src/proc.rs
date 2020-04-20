use chrono::Local;
use proc_maps::{get_process_maps, Pid};
use procfs::process::Process;
use std::error::Error;

pub fn get_process_average_cpu(pid: i64) -> Result<f32, Box<dyn Error>> {
    debug!("Getting CPU usage for PID {}", pid);

    let p = Process::new(pid as i32)?;
    let stat = p.stat()?;
    let total_ticks = stat.utime + stat.stime + stat.cutime as u64 + stat.cstime as u64;
    let seconds = (Local::now() - stat.starttime()?).num_seconds() as f32;
    let cpu = 100.0 * ((total_ticks as f32 / procfs::ticks_per_second()? as f32) / seconds);

    debug!("Calulated CPU usage: {}", cpu);
    Ok(cpu)
}

pub fn get_process_memory_bytes(memory_backend: &str, pid: i64) -> Result<usize, Box<dyn Error>> {
    debug!(
        "Getting memory usage for PID {} with backend {}",
        pid, memory_backend
    );
    let mut memory = 0;

    if memory_backend.eq("procmaps") {
        let maps = get_process_maps(pid as Pid)?;
        for map in maps {
            memory += map.size();
        }
    } else {
        let p = Process::new(pid as i32)?;
        let stat = p.stat()?;
        memory = match memory_backend {
            "rss" => stat.rss_bytes() as usize,
            "vsz" => stat.vsize as usize,
            &_ => panic!("Got unknown memory backend"), // Should never happen since we check this in main
        }
    }

    debug!("Calulated memory usage: {}", memory);
    Ok(memory)
}
