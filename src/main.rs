use bytesize::ByteSize;
use dockworker::container::{Container, ContainerFilters};
use dockworker::Docker;
use itertools::{fold, join};
use proc_maps::{get_process_maps, Pid};
use procfs::process::Process;
use fancy_regex::Regex;
use structopt::StructOpt;
use tabled::{table, Tabled};

#[derive(Tabled)]
struct ContainerStats {
    memory: ByteSize,
    name: String,
    id: String,
}

#[derive(Tabled)]
struct ContainerGroup {
    memory: ByteSize,
    containers: i32,
    fix: String,
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

    /// Filters container names by a regular expression
    #[structopt(long, short)]
    regex: Option<String>,

    /// The way the used memory is calculated. Options are: "procmaps" (cross-platform), "rss" and "vsz" (both linux).
    #[structopt(long, short, default_value = "procmaps")]
    memory_backend: String,
}

fn main() {
    let opt = Opt::from_args();
    if !["procmaps".to_owned(), "rss".to_owned(), "vsz".to_owned()].contains(&opt.memory_backend) {
        println!("Error: unsupported memory backend");
        return;
    }

    let docker = Docker::connect_with_defaults().unwrap();
    match docker.list_containers(None, None, None, ContainerFilters::new()) {
        Ok(result) => handle_containers(&opt, docker, result),
        Err(e) => println!("Error connecting to docker daemon: {}", e),
    };

    fn handle_containers(opt: &Opt, docker: Docker, containers: Vec<Container>) {
        let mut all_stats = gather_stats(opt, docker, containers);

        if let Some(regex) = &opt.regex {
            all_stats = filter(all_stats, regex);
        }

        if opt.total {
            let total = fold(&all_stats, 0, |i, stats| i + stats.memory.as_u64());
            println!("Total: {} ({} B)", ByteSize::b(total), total);
            return;
        }

        if opt.group_by_prefix || opt.group_by_suffix {
            let mut grouped_stats = Vec::<ContainerGroup>::new();
            for stat in &all_stats {
                let mut split_name = stat.name.split(opt.delimiter);
                let fix = match opt.group_by_prefix {
                    true => split_name.nth(0).unwrap_or(&stat.name).to_string(),
                    false => split_name.last().unwrap_or(&stat.name).to_string(),
                };

                let mut found = false;
                for group in &mut grouped_stats {
                    if group.fix.eq(&fix) {
                        group.memory = group.memory + stat.memory;
                        group.containers += 1;
                        found = true;
                    }
                }

                if !found {
                    grouped_stats.push(ContainerGroup {
                        memory: stat.memory,
                        containers: 1,
                        fix: String::from(&fix),
                    });
                }
            }

            if opt.sort {
                grouped_stats.sort_by(|a, b| b.memory.cmp(&a.memory));
            }
            println!("{}", table(&grouped_stats));
            return;
        }

        if opt.sort {
            all_stats.sort_by(|a, b| b.memory.cmp(&a.memory));
        }
        println!("{}", table(&all_stats));
    }

    fn gather_stats(opt: &Opt, docker: Docker, containers: Vec<Container>) -> Vec<ContainerStats> {
        let mut all_stats = Vec::new();
        for container in containers {
            let mut memory = 0;
            let mut pids = Vec::<i64>::new();

            if opt.top {
                let processes = docker.processes(&container.Id).unwrap();
                for p in processes {
                    pids.push(p.pid.parse::<i64>().unwrap());
                }
            }
            else {
                pids.push(docker.container_info(&container.Id).unwrap().State.Pid);
            }

            for pid in pids {
                memory += get_process_memory_bytes(opt, pid) as u64;
            }

            all_stats.push(ContainerStats {
                id: container.Id,
                name: join(container.Names, ", "),
                memory: ByteSize::b(memory),
            })
        }
        all_stats
    }

    fn get_process_memory_bytes(opt: &Opt, pid: i64) -> usize {
        let mut memory = 0;

        if opt.memory_backend.eq("procmaps") {
            let maps = get_process_maps(pid as Pid).unwrap();
            for map in maps {
                memory += map.size();
            }
        } else {
            let p = Process::new(pid as i32).expect("Failed to access process");
            let stat = p.stat().expect("Failed to get process stat");
            memory = match opt.memory_backend.as_ref() {
                "rss" => stat.rss_bytes() as usize,
                "vsz" => stat.vsize as usize,
                &_ => panic!("Got unknown memory backend"),
            }
        }
        memory
    }

    fn filter(stats: Vec<ContainerStats>, pattern: &str) -> Vec<ContainerStats> {
        let re = Regex::new(pattern).expect("Failed to create Regex pattern!");
        stats
            .into_iter()
            .filter(|s| re.is_match(&s.name).unwrap())
            .collect()
    }
}
