# Termux Installation and Initial Setup

This guide covers installing Termux on an Android device and performing initial configuration.

## Installing Termux

### Source: F-Droid (Required)

**Important**: Install Termux from F-Droid, not the Google Play Store. The Play Store version is outdated and no longer maintained.

#### Option 1: F-Droid App

1. Install F-Droid from [f-droid.org](https://f-droid.org)
2. Open F-Droid and search for "Termux"
3. Install Termux and Termux:API (for device integration)

#### Option 2: Direct APK Download

Download the latest APK directly from the Termux GitHub releases:

```
https://github.com/termux/termux-app/releases
```

Download the appropriate APK for your device architecture (usually `arm64-v8a`).

### Verify Installation

Open Termux. You should see a terminal prompt:

```
$ _
```

## Initial System Update

First-time setup requires updating package repositories:

```bash
# Update package lists and upgrade installed packages
pkg update && pkg upgrade -y

# Install essential utilities
pkg install -y \
    coreutils \
    findutils \
    grep \
    sed \
    gawk \
    which \
    man \
    less \
    nano \
    vim
```

## Storage Access

Grant Termux access to device storage:

```bash
termux-setup-storage
```

This creates symlinks in `~/storage/`:

| Path | Description |
|------|-------------|
| `~/storage/shared` | Main internal storage |
| `~/storage/downloads` | Downloads folder |
| `~/storage/dcim` | Camera photos |
| `~/storage/music` | Music folder |
| `~/storage/external-1` | SD card (if present) |

## Termux Properties

Configure Termux behavior by editing properties:

```bash
mkdir -p ~/.termux
nano ~/.termux/termux.properties
```

Recommended settings:

```properties
# Use black background
use-black-ui = true

# Extra keys row for common keys
extra-keys = [['ESC','/','-','HOME','UP','END','PGUP'],['TAB','CTRL','ALT','LEFT','DOWN','RIGHT','PGDN']]

# Bell behavior
bell-character = ignore

# Keyboard shortcuts
shortcut.create-session = ctrl + t
shortcut.next-session = ctrl + n
shortcut.previous-session = ctrl + p
shortcut.rename-session = ctrl + r
```

Reload properties:

```bash
termux-reload-settings
```

## Background Execution

By default, Android aggressively kills background apps. Configure Termux to run reliably:

### Acquire Wake Lock

Prevents Android from killing Termux when in background:

```bash
termux-wake-lock
```

Add to shell profile for automatic activation:

```bash
echo 'termux-wake-lock' >> ~/.bashrc
```

### Battery Optimization

Disable battery optimization for Termux:

1. Android Settings → Apps → Termux
2. Battery → Unrestricted (or "Don't optimize")

### Notification Channel

Keep Termux notification visible to maintain background priority:

1. Long-press Termux notification
2. Ensure notifications are enabled

## Shell Configuration

### Bash Profile

Create a proper bash profile:

```bash
cat > ~/.bashrc << 'EOF'
# Termux bash configuration

# History
HISTSIZE=10000
HISTFILESIZE=20000
HISTCONTROL=ignoreboth:erasedups
shopt -s histappend

# Prompt
PS1='\[\e[32m\]\u@termux\[\e[0m\]:\[\e[34m\]\w\[\e[0m\]\$ '

# Aliases
alias ll='ls -la'
alias la='ls -A'
alias l='ls -CF'
alias ..='cd ..'
alias ...='cd ../..'

# Path
export PATH="$HOME/bin:$HOME/.cargo/bin:$PATH"

# Editor
export EDITOR=nano
export VISUAL=nano

# Wake lock for background execution
termux-wake-lock 2>/dev/null

# Welcome message
echo "Termux ready - $(uname -m) - $(date)"
EOF
```

Source the profile:

```bash
source ~/.bashrc
```

### Create bin Directory

For custom scripts:

```bash
mkdir -p ~/bin
```

## System Information

Verify the environment:

```bash
# Architecture (should be aarch64 for ARM64)
uname -m

# Android version
getprop ro.build.version.release

# Available memory
free -h

# Storage space
df -h ~

# CPU info
cat /proc/cpuinfo | grep -E "^(processor|model name|Hardware)" | head -10
```

## Termux-Specific Notes

### No Root Required

Termux runs in userspace without root. This means:

- Cannot bind to ports < 1024 (use 8022 for SSH, not 22)
- Cannot access `/system` or other protected paths
- All files are in `$HOME` (`/data/data/com.termux/files/home`)

### Package Manager

Termux uses `pkg` (wrapper for `apt`):

```bash
pkg search <term>     # Search packages
pkg install <pkg>     # Install package
pkg uninstall <pkg>   # Remove package
pkg list-installed    # List installed packages
pkg upgrade           # Upgrade all packages
```

### Common Issues

#### "Permission denied" on scripts

```bash
chmod +x script.sh
```

#### Package database locked

Another Termux session may be updating:

```bash
rm -f $PREFIX/var/lib/dpkg/lock
rm -f $PREFIX/var/lib/dpkg/lock-frontend
pkg update
```

#### "Repository is under maintenance"

Mirrors may be temporarily unavailable. Change mirror:

```bash
termux-change-repo
```

## Next Steps

Continue to [02-ssh-configuration.md](02-ssh-configuration.md) to set up remote SSH access.
