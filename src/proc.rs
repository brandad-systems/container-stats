use chrono::Local;
use proc_maps::{get_process_maps, Pid};
use procfs::process::Process;

pub fn get_process_average_cpu(pid: i64) -> f32 {
    let p = Process::new(pid as i32).expect("Failed to access process");
    let stat = p.stat().expect("Failed to get process stat");
    let total_ticks = stat.utime + stat.stime + stat.cutime as u64 + stat.cstime as u64;
    let seconds = (Local::now() - stat.starttime().unwrap()).num_seconds() as f32;
    100.0 * ((total_ticks as f32 / procfs::ticks_per_second().unwrap() as f32) / seconds)
}

pub fn get_process_memory_bytes(memory_backend: &str, pid: i64) -> usize {
    let mut memory = 0;

    if memory_backend.eq("procmaps") {
        let maps = get_process_maps(pid as Pid).unwrap();
        for map in maps {
            memory += map.size();
        }
    } else {
        let p = Process::new(pid as i32).expect("Failed to access process");
        let stat = p.stat().expect("Failed to get process stat");
        memory = match memory_backend {
            "rss" => stat.rss_bytes() as usize,
            "vsz" => stat.vsize as usize,
            &_ => panic!("Got unknown memory backend"),
        }
    }
    memory
}
