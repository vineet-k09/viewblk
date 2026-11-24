use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::ffi::CString;
use std::fs;
use std::mem::MaybeUninit;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PartitionCustomInfo {
    pub name: String,
    pub description: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DriveCustomInfo {
    pub name: String,
    pub drive_type: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub drives: HashMap<String, DriveCustomInfo>,
    pub partitions: HashMap<String, PartitionCustomInfo>,
}

#[derive(Debug, Clone)]
pub struct PartitionInfo {
    pub dev_node: String,
    pub dev_path: String,
    pub sys_size_bytes: u64,
    pub fstype: String,
    pub label: String,
    pub mountpoints: Vec<String>,
    pub is_mounted: bool,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub use_percent: f64,
    pub custom_name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct DriveInfo {
    pub dev_path: String,
    pub dev_name: String,
    pub is_ssd: bool,
    pub total_size_bytes: u64,
    pub custom_name: String,
    pub partitions: Vec<PartitionInfo>,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!(
            "\x1b[1;36mviewblk\x1b[0m - Human-friendly disk & partition layout CLI

\x1b[1mUSAGE:\x1b[0m
    viewblk

\x1b[1mCONFIGURATION:\x1b[0m
    \x1b[33m~/.config/viewblk/config.toml\x1b[0m
"
        );
        return;
    }

    let config_path = get_config_path();
    let config = load_or_create_config(&config_path);
    let drives = scan_drives(&config);
    print_cli_tree(&drives, &config_path);
}

fn get_config_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".config").join("viewblk").join("config.toml")
}

fn load_or_create_config(path: &Path) -> Config {
    if path.exists() {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(cfg) = toml::from_str::<Config>(&content) {
                return cfg;
            }
        }
    }

    let default_config = build_default_config();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(toml_str) = toml::to_string_pretty(&default_config) {
        let header = "# ViewBlk Configuration File\n# Map drives and partitions to human-friendly names and descriptions.\n\n";
        let _ = fs::write(path, format!("{}{}", header, toml_str));
    }
    default_config
}

fn build_default_config() -> Config {
    let mut drives = HashMap::new();
    let mut partitions = HashMap::new();

    drives.insert(
        "nvme0n1".to_string(),
        DriveCustomInfo {
            name: "Primary NVMe SSD (256 GB)".to_string(),
            drive_type: Some("SSD".to_string()),
            description: Some("Fast system drive for OS & active apps".to_string()),
        },
    );

    drives.insert(
        "sda".to_string(),
        DriveCustomInfo {
            name: "Secondary HDD (1 TB)".to_string(),
            drive_type: Some("HDD".to_string()),
            description: Some("High-capacity storage for projects & media".to_string()),
        },
    );

    partitions.insert(
        "/dev/sda4".to_string(),
        PartitionCustomInfo {
            name: "Day-to-day Work".to_string(),
            description: Some("Active projects, code & daily workspace (/mnt/bridge)".to_string()),
            category: Some("Work".to_string()),
        },
    );

    partitions.insert(
        "/dev/sda3".to_string(),
        PartitionCustomInfo {
            name: "Brain Fuse".to_string(),
            description: Some("Old Windows dev work & legacy archives".to_string()),
            category: Some("Archives".to_string()),
        },
    );

    partitions.insert(
        "/dev/sda2".to_string(),
        PartitionCustomInfo {
            name: "Library".to_string(),
            description: Some("Personal files, media, photos & backups".to_string()),
            category: Some("Personal".to_string()),
        },
    );

    partitions.insert(
        "/dev/sda5".to_string(),
        PartitionCustomInfo {
            name: "FatEx Shared".to_string(),
            description: Some("ExFAT cross-platform partition".to_string()),
            category: Some("Storage".to_string()),
        },
    );

    partitions.insert(
        "/dev/nvme0n1p7".to_string(),
        PartitionCustomInfo {
            name: "Fedora OS & Home".to_string(),
            description: Some("Linux system root (/) and user home (/home) [Btrfs Pool]".to_string()),
            category: Some("System".to_string()),
        },
    );

    partitions.insert(
        "/dev/nvme0n1p2".to_string(),
        PartitionCustomInfo {
            name: "Fedora OS Pool".to_string(),
            description: Some("Pooled Btrfs storage for Fedora root & home (/)".to_string()),
            category: Some("System".to_string()),
        },
    );

    partitions.insert(
        "/dev/nvme0n1p6".to_string(),
        PartitionCustomInfo {
            name: "Linux Boot".to_string(),
            description: Some("Kernel image & boot files (/boot)".to_string()),
            category: Some("System".to_string()),
        },
    );

    partitions.insert(
        "/dev/nvme0n1p1".to_string(),
        PartitionCustomInfo {
            name: "EFI System".to_string(),
            description: Some("UEFI Bootloader partition (/boot/efi)".to_string()),
            category: Some("System".to_string()),
        },
    );

    Config { drives, partitions }
}

fn scan_drives(config: &Config) -> Vec<DriveInfo> {
    let mounts_map = parse_mounts();
    let dev_labels = scan_dev_labels();

    let mut result = Vec::new();

    let block_path = Path::new("/sys/class/block");
    if !block_path.exists() {
        return result;
    }

    let entries = match fs::read_dir(block_path) {
        Ok(e) => e,
        Err(_) => return result,
    };

    let mut raw_drives: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("loop") || name.starts_with("zram") || name.starts_with("sr") || name.starts_with("ram") {
            continue;
        }

        let is_partition = entry.path().join("partition").exists();
        if !is_partition {
            raw_drives.entry(name).or_insert_with(Vec::new);
        }
    }

    if let Ok(entries) = fs::read_dir(block_path) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.path().join("partition").exists() {
                for parent_name in raw_drives.keys().cloned().collect::<Vec<_>>() {
                    if name.starts_with(&parent_name) {
                        raw_drives.get_mut(&parent_name).unwrap().push(name.clone());
                        break;
                    }
                }
            }
        }
    }

    for (parent_name, part_names) in raw_drives {
        let sys_parent = block_path.join(&parent_name);

        let is_rotational = read_sys_file(&sys_parent.join("queue/rotational"))
            .unwrap_or_else(|| "1".into()) == "1";
        let is_ssd = !is_rotational;

        let total_size_bytes = read_sys_size(&sys_parent.join("size"));

        let custom_drive_info = config.drives.get(&parent_name);
        let drive_custom_name = custom_drive_info
            .map(|d| d.name.clone())
            .unwrap_or_else(|| {
                if is_ssd {
                    format!("SSD ({})", parent_name)
                } else {
                    format!("HDD ({})", parent_name)
                }
            });

        let mut partitions = Vec::new();
        let mut sorted_parts = part_names.clone();
        sorted_parts.sort();

        for part_node in sorted_parts {
            let dev_path = format!("/dev/{}", part_node);
            let sys_part = block_path.join(&part_node);
            let part_size_bytes = read_sys_size(&sys_part.join("size"));

            let mountpoints = mounts_map.get(&dev_path).cloned().unwrap_or_default();
            let is_mounted = !mountpoints.is_empty();

            let label = dev_labels.get(&dev_path).cloned().unwrap_or_default();

            let mut fstype = String::new();
            let mut total_bytes = part_size_bytes;
            let mut used_bytes = 0;
            let mut free_bytes = 0;
            let mut use_percent = 0.0;

            if is_mounted {
                if let Some(mp) = mountpoints.first() {
                    if let Some(stat) = get_statvfs(mp) {
                        total_bytes = stat.total_bytes;
                        used_bytes = stat.used_bytes;
                        free_bytes = stat.avail_bytes;
                        if total_bytes > 0 {
                            use_percent = (used_bytes as f64 / total_bytes as f64) * 100.0;
                        }
                    }
                }
                fstype = get_fstype_from_mounts(&dev_path);
            }

            let custom_part_info = config.partitions.get(&dev_path);
            let custom_name = custom_part_info
                .map(|p| p.name.clone())
                .unwrap_or_else(|| {
                    if !label.is_empty() {
                        label.clone()
                    } else if is_mounted {
                        format!("Mounted at {}", mountpoints.join(", "))
                    } else {
                        format!("Partition {}", part_node)
                    }
                });

            let description = custom_part_info
                .and_then(|p| p.description.clone())
                .unwrap_or_default();

            partitions.push(PartitionInfo {
                dev_node: part_node,
                dev_path,
                sys_size_bytes: part_size_bytes,
                fstype,
                label,
                mountpoints,
                is_mounted,
                total_bytes,
                used_bytes,
                free_bytes,
                use_percent,
                custom_name,
                description,
            });
        }

        result.push(DriveInfo {
            dev_path: format!("/dev/{}", parent_name),
            dev_name: parent_name,
            is_ssd,
            total_size_bytes,
            custom_name: drive_custom_name,
            partitions,
        });
    }

    result
}

fn parse_mounts() -> HashMap<String, Vec<String>> {
    let mut mounts = HashMap::new();
    if let Ok(content) = fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let dev = parts[0];
                let mountpoint = parts[1].replace("\\040", " ").replace("\\011", "\t");
                if dev.starts_with("/dev/") {
                    mounts.entry(dev.to_string()).or_insert_with(Vec::new).push(mountpoint);
                }
            }
        }
    }

    // Detect multi-device Btrfs pools via /sys/fs/btrfs/
    let btrfs_sys = Path::new("/sys/fs/btrfs");
    if let Ok(entries) = fs::read_dir(btrfs_sys) {
        for entry in entries.flatten() {
            let dev_dir = entry.path().join("devices");
            if dev_dir.is_dir() {
                if let Ok(dev_entries) = fs::read_dir(&dev_dir) {
                    let pool_devs: Vec<String> = dev_entries
                        .flatten()
                        .map(|d| format!("/dev/{}", d.file_name().to_string_lossy()))
                        .collect();

                    let mut pool_mounts = Vec::new();
                    for d in &pool_devs {
                        if let Some(mps) = mounts.get(d) {
                            for mp in mps {
                                if !pool_mounts.contains(mp) {
                                    pool_mounts.push(mp.clone());
                                }
                            }
                        }
                    }

                    if !pool_mounts.is_empty() {
                        for d in pool_devs {
                            mounts.entry(d).or_insert_with(|| pool_mounts.clone());
                        }
                    }
                }
            }
        }
    }

    mounts
}

fn get_fstype_from_mounts(dev_path: &str) -> String {
    if let Ok(content) = fs::read_to_string("/proc/mounts") {
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == dev_path {
                return parts[2].to_string();
            }
        }
    }

    // Check if device is part of a Btrfs pool
    if let Some(dev_name) = dev_path.strip_prefix("/dev/") {
        let btrfs_sys = Path::new("/sys/fs/btrfs");
        if let Ok(entries) = fs::read_dir(btrfs_sys) {
            for entry in entries.flatten() {
                if entry.path().join("devices").join(dev_name).exists() {
                    return "btrfs".to_string();
                }
            }
        }
    }

    String::new()
}

fn scan_dev_labels() -> HashMap<String, String> {
    let mut map = HashMap::new();
    let by_label_path = Path::new("/dev/disk/by-label");
    if let Ok(entries) = fs::read_dir(by_label_path) {
        for entry in entries.flatten() {
            let label = entry.file_name().to_string_lossy().to_string().replace("\\x20", " ");
            if let Ok(target) = fs::read_link(entry.path()) {
                if let Ok(canonical) = fs::canonicalize(by_label_path.join(target)) {
                    map.insert(canonical.to_string_lossy().to_string(), label);
                }
            }
        }
    }
    map
}

struct StatResult {
    total_bytes: u64,
    used_bytes: u64,
    avail_bytes: u64,
}

fn get_statvfs(path: &str) -> Option<StatResult> {
    let c_path = CString::new(path).ok()?;
    let mut stat = MaybeUninit::<libc::statvfs>::uninit();

    let res = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
    if res == 0 {
        let stat = unsafe { stat.assume_init() };
        let block_size = stat.f_frsize as u64;
        let total_bytes = stat.f_blocks as u64 * block_size;
        let free_bytes = stat.f_bfree as u64 * block_size;
        let avail_bytes = stat.f_bavail as u64 * block_size;
        let used_bytes = total_bytes.saturating_sub(free_bytes);

        Some(StatResult {
            total_bytes,
            used_bytes,
            avail_bytes,
        })
    } else {
        None
    }
}

fn read_sys_file(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn read_sys_size(path: &Path) -> u64 {
    read_sys_file(path)
        .and_then(|s| s.parse::<u64>().ok())
        .map(|sectors| sectors * 512)
        .unwrap_or(0)
}

fn format_bytes(bytes: u64) -> String {
    let kb = 1024.0;
    let mb = kb * 1024.0;
    let gb = mb * 1024.0;
    let tb = gb * 1024.0;

    let b = bytes as f64;
    if b >= tb {
        format!("{:.1} TB", b / tb)
    } else if b >= gb {
        format!("{:.1} GB", b / gb)
    } else if b >= mb {
        format!("{:.1} MB", b / mb)
    } else if b >= kb {
        format!("{:.1} KB", b / kb)
    } else {
        format!("{} B", bytes)
    }
}

fn draw_bar(pct: f64, width: usize) -> String {
    let clamped = pct.clamp(0.0, 100.0);
    let filled_len = ((clamped / 100.0) * width as f64).round() as usize;
    let empty_len = width.saturating_sub(filled_len);

    let bar_char = "█";
    let empty_char = "░";

    format!(
        "[{}{}] {:>3.0}%",
        bar_char.repeat(filled_len),
        empty_char.repeat(empty_len),
        clamped
    )
}

fn print_cli_tree(drives: &[DriveInfo], config_path: &Path) {
    println!("\x1b[1;36m================================================================================\x1b[0m");
    println!("\x1b[1;37m  VIEWBLK - Human-Friendly Drive & Partition Layout\x1b[0m");
    println!("\x1b[1;36m================================================================================\x1b[0m");

    for (d_idx, drive) in drives.iter().enumerate() {
        let badge = if drive.is_ssd {
            "\x1b[1;33m🚀 SSD\x1b[0m"
        } else {
            "\x1b[1;34m💾 HDD\x1b[0m"
        };

        println!(
            "\n{} \x1b[1;37m{}\x1b[0m \x1b[90m[{}] ({})\x1b[0m",
            badge,
            drive.custom_name,
            drive.dev_name,
            format_bytes(drive.total_size_bytes)
        );

        let part_count = drive.partitions.len();
        for (p_idx, part) in drive.partitions.iter().enumerate() {
            let is_last = p_idx == part_count - 1;
            let branch = if is_last { "└──" } else { "├──" };
            let pipe = if is_last { "   " } else { "│  " };

            let mount_str = if part.is_mounted {
                format!("\x1b[32mMounted:\x1b[0m {}", part.mountpoints.join(", "))
            } else {
                "\x1b[90m[Unmounted]\x1b[0m".to_string()
            };

            let fstype_str = if !part.fstype.is_empty() {
                format!("\x1b[35m[{}]\x1b[0m ", part.fstype)
            } else {
                String::new()
            };

            println!(
                "{} \x1b[1;32m{:<10}\x1b[0m {}{:<10} | {}",
                branch,
                part.dev_node,
                fstype_str,
                format_bytes(part.sys_size_bytes),
                mount_str
            );

            println!(
                "{} │ \x1b[1;36mName:\x1b[0m \x1b[1;37m{}\x1b[0m",
                pipe, part.custom_name
            );

            if part.is_mounted {
                let bar = draw_bar(part.use_percent, 20);
                let color_code = if part.use_percent > 90.0 {
                    "\x1b[1;31m"
                } else if part.use_percent > 75.0 {
                    "\x1b[1;33m"
                } else {
                    "\x1b[1;32m"
                };

                println!(
                    "{} │ \x1b[1;36mUsage:\x1b[0m {}{}\x1b[0m ({}/{} used, {} free)",
                    pipe,
                    color_code,
                    bar,
                    format_bytes(part.used_bytes),
                    format_bytes(part.total_bytes),
                    format_bytes(part.free_bytes)
                );
            }

            if !part.description.is_empty() {
                println!("{} └ \x1b[90mPurpose: {}\x1b[0m", pipe, part.description);
            } else {
                println!("{} └ \x1b[90mPurpose: -\x1b[0m", pipe);
            }

            if !is_last {
                println!("{}", pipe);
            }
        }

        if d_idx < drives.len() - 1 {
            println!("\x1b[90m--------------------------------------------------------------------------------\x1b[0m");
        }
    }

    println!("\n\x1b[90mConfig path: {}\x1b[0m", config_path.display());
}
