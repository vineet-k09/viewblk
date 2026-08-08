# ViewBlk (`viewblk`)

A super lightweight (~840KB), ultra-fast CLI utility in Rust that renders a human-friendly breakdown of your drives and partitions directly in your terminal.

Instead of cryptic device nodes like `sda4` or `nvme0n1p7`, `viewblk` displays meaningful names, purposes, mount paths, disk usage percentages with visual progress bars, and automatically distinguishes between **🚀 NVMe SSDs** and **💾 HDDs**.

---

## Features

- **🚀 Ultra-lightweight & Instant**: Single ~840KB stripped Rust binary with < 2ms execution time and zero background daemons.
- **🏷️ Human-friendly Naming**: Maps drives and partitions to custom names and descriptions (`Day-to-day Work`, `Brain Fuse`, `Library`, `Fedora OS & Home`).
- **🎨 Visual Progress Bars**: Color-coded usage bars (`[████████████░░░░░░░░] 62%`).
- **⚙️ Customizable Config**: Configuration stored at `~/.config/viewblk/config.toml`.

---

## Usage

Simply run:

```bash
viewblk
```

---

## Configuration

Edit `~/.config/viewblk/config.toml` to customize partition names or descriptions:

```toml
[partitions."/dev/sda4"]
name = "Day-to-day Work"
description = "Active projects, code & daily workspace (/mnt/bridge)"

[partitions."/dev/sda3"]
name = "Brain Fuse"
description = "Old Windows dev work & legacy archives"

[partitions."/dev/sda2"]
name = "Library"
description = "Personal files, media, photos & backups"
```
