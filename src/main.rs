mod proc;

#[macro_use]
extern crate log;

use bytesize::ByteSize;
use dockworker::container::{Container, ContainerFilters};
use dockworker::Docker;
use fancy_regex::Regex;
use itertools::{fold, join, Itertools};
use serde::{Serialize, Serializer};
use std::fmt;
use structopt::StructOpt;
use tabled::{table, Tabled};

#[derive(Tabled, Serialize, Debug)]
struct ContainerStats {
    memory: SerializableByteSize,
    average_percent_cpu: f32,
    name: String,
    id: String,
}

#[derive(Tabled, Serialize)]
struct ContainerGroup {
    memory: SerializableByteSize,
    average_percent_cpu: f32,
    containers: i32,
    fix: String,
}

#[derive(Debug, Clone, Copy)]
struct SerializableByteSize(ByteSize);

impl Serialize for SerializableByteSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(self.0.as_u64())
    }
}

impl fmt::Display for SerializableByteSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(StructOpt, Debug)]
#[structopt(name = "container-stats", author, about)]
struct Opt {
    /// Prints total memory used by containers
    #[structopt(short, long)]
    total: bool,

    /// Sorts containers by memory used
    #[structopt(short, long)]
    sort: bool,

    /// Group containers by prefix
    #[structopt(long)]
    group_by_prefix: bool,

    /// Group containers by suffix
    #[structopt(long)]
    group_by_suffix: bool,

    /// Delimiter for grouping
    #[structopt(long, short, default_value = "-")]
    delimiter: char,

    /// Use docker top. Not supported on windows & significantly slower, but correctly detects multiple processes per container
    #[structopt(long)]
    top: bool,

    /// Print as json instead of a table
    #[structopt(long)]
    json: bool,

    /// Filters container names by a regular expression
    #[structopt(long, short)]
    regex: Option<String>,

    /// The way the used memory is calculated. Options are: "procmaps" (cross-platform), "rss" and "vsz" (both linux).
    #[structopt(long, short, default_value = "procmaps")]
    memory_backend: String,

    /// The logging level, in case the RUST_LOG environment variable cannot be set.
    #[structopt(long)]
    debug: Option<String>,
}

fn main() {
    let opt = Opt::from_args();

    if let Some(level) = &opt.debug {
        env_logger::init_from_env(env_logger::Env::default().default_filter_or(level));
    } else {
        env_logger::init();
    }

    debug!("Running with arguments: {:#?}", opt);

    if !["procmaps".to_owned(), "rss".to_owned(), "vsz".to_owned()].contains(&opt.memory_backend) {
        error!("Error: unsupported memory backend");
        return;
    }

    info!("Attempting to connect to docker daemon");
    let docker = Docker::connect_with_defaults().unwrap();
    match docker.list_containers(None, None, None, ContainerFilters::new()) {
        Ok(result) => {
            let wanted_state = "running";
            debug!("Filtering containers {} by Status (want `{}`)", result.len(), wanted_state);
            let v = result
                .iter()
                .filter(|c| c.State.eq(wanted_state))
                .map(|c| c.to_owned())
                .collect_vec();
            handle_containers(&opt, docker, v);
        }
        Err(e) => error!("Error connecting to docker daemon: {}", e),
    }
}

fn handle_containers(opt: &Opt, docker: Docker, containers: Vec<Container>) {
    info!("Processing {} containers", containers.len());
    let mut all_stats = gather_stats(opt, docker, containers);
    debug!("All stats gathered: {:#?}", all_stats);

    if let Some(regex) = &opt.regex {
        debug!("Filtering {} stats by regex {}", all_stats.len(), regex);
        all_stats = filter(all_stats, regex);
        debug!("Remaining: {} stats", all_stats.len());
    }

    if opt.total {
        debug!("Calulating total memory usage (prints immediately)");
        let total = fold(&all_stats, 0, |i, stats| i + stats.memory.0.as_u64());
        println!("Total: {} ({} B)", ByteSize::b(total), total);
        return;
    }

    if opt.group_by_prefix || opt.group_by_suffix {
        info!(
            "Grouping {} stats (by prefix: {})",
            all_stats.len(),
            opt.group_by_prefix
        );
        let mut grouped_stats = Vec::<ContainerGroup>::new();
        for stat in &all_stats {
            debug!("Processing stats {:#?}", stat);
            let mut split_name = stat.name.split(opt.delimiter);
            let fix = if opt.group_by_prefix {
                split_name.next().unwrap_or(&stat.name).to_string()
            } else {
                split_name.last().unwrap_or(&stat.name).to_string()
            };

            let mut found = false;
            for group in &mut grouped_stats {
                if group.fix.eq(&fix) {
                    group.memory = SerializableByteSize(group.memory.0 + stat.memory.0);
                    group.average_percent_cpu += stat.average_percent_cpu;
                    group.containers += 1;
                    found = true;
                }
            }

            if !found {
                grouped_stats.push(ContainerGroup {
                    memory: stat.memory,
                    average_percent_cpu: stat.average_percent_cpu,
                    containers: 1,
                    fix: String::from(&fix),
                });
            }
        }

        if opt.sort {
            debug!("Sorting {} stats", grouped_stats.len());
            grouped_stats.sort_by(|a, b| b.memory.0.cmp(&a.memory.0));
        }
        print(opt, &grouped_stats);
        return;
    }

    if opt.sort {
        debug!("Sorting {} stats", all_stats.len());
        all_stats.sort_by(|a, b| b.memory.0.cmp(&a.memory.0));
    }
    print(opt, &all_stats);
}

fn gather_stats(opt: &Opt, docker: Docker, containers: Vec<Container>) -> Vec<ContainerStats> {
    let mut all_stats = Vec::new();
    for container in containers {
        info!("Gathering stats for container with ID {}", container.Id);
        let mut memory = 0;
        let mut average_percent_cpu = 0.0;
        let mut pids = Vec::<i64>::new();

        if opt.top {
            let processes = docker.processes(&container.Id).unwrap();
            for p in processes {
                pids.push(p.pid.parse::<i64>().unwrap());
            }
        } else {
            pids.push(docker.container_info(&container.Id).unwrap().State.Pid);
        }
        debug!("Found {} processes: {:#?}", pids.len(), pids);

        for pid in pids {
            match proc::get_process_memory_bytes(&opt.memory_backend, pid) {
                Ok(mem) => memory += mem as u64,
                Err(e) => error!(
                    "Failed to get memory for process {} (from container {}) due to {}",
                    pid, container.Id, e
                ),
            };
            match proc::get_process_average_cpu(pid) {
                Ok(cpu) => average_percent_cpu += cpu,
                Err(e) => error!(
                    "Failed to get average CPU for process {} (from container {}) due to {}",
                    pid, container.Id, e
                ),
            };
        }

        all_stats.push(ContainerStats {
            id: container.Id,
            name: join(container.Names, ", "),
            memory: SerializableByteSize(ByteSize::b(memory)),
            average_percent_cpu,
        })
    }
    all_stats
}

fn filter(stats: Vec<ContainerStats>, pattern: &str) -> Vec<ContainerStats> {
    let re = Regex::new(pattern).expect("Failed to create Regex pattern!");
    stats
        .into_iter()
        .filter(|s| re.is_match(&s.name).unwrap())
        .collect()
}

fn print(opt: &Opt, to_print: &[impl Tabled + Serialize]) {
    if opt.json {
        debug!("Printing as json");
        println!("{}", serde_json::to_string_pretty(to_print).unwrap())
    } else {
        debug!("Printing as table");
        println!("{}", table(to_print));
    }
}
