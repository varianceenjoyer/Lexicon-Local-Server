# Development Environment Setup

This guide covers setting up Git, Rust, and build dependencies for Lexicon development on Termux.

## Git Configuration

### Install Git

```bash
pkg install -y git
```

### Configure Identity

```bash
git config --global user.name "Your Name"
git config --global user.email "your.email@example.com"
```

### Configure Defaults

```bash
# Default branch name
git config --global init.defaultBranch main

# Pull behavior
git config --global pull.rebase false

# Editor
git config --global core.editor nano

# Credential caching (15 min)
git config --global credential.helper cache
```

### SSH Key for GitHub/GitLab

Generate a key specifically for the device:

```bash
ssh-keygen -t ed25519 -C "termux-dev@android"
```

Display public key to add to GitHub/GitLab:

```bash
cat ~/.ssh/id_ed25519.pub
```

Add to GitHub: Settings → SSH and GPG keys → New SSH key

Test connection:

```bash
ssh -T git@github.com
# Expected: Hi username! You've successfully authenticated...
```

## Rust Installation

### Install Rust via rustup

```bash
# Install build dependencies first
pkg install -y binutils

# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

Load cargo environment:

```bash
source "$HOME/.cargo/env"
```

Add to `.bashrc` for persistence:

```bash
echo 'source "$HOME/.cargo/env"' >> ~/.bashrc
```

### Verify Installation

```bash
rustc --version
# rustc 1.XX.X (...)

cargo --version
# cargo 1.XX.X (...)

rustup --version
# rustup 1.XX.X (...)
```

### Update Rust

```bash
rustup update
```

## Build Dependencies

Install system packages required for Lexicon build:

```bash
pkg install -y \
    build-essential \
    cmake \
    pkg-config \
    libclang \
    openssl \
    openssl-tool
```

### Package Details

| Package | Purpose |
|---------|---------|
| `build-essential` | C/C++ compiler, make, binutils |
| `cmake` | Build system for whisper-rs |
| `pkg-config` | Library discovery |
| `libclang` | Required by bindgen for FFI |
| `openssl` | TLS support |

### Set Environment Variables

Add to `~/.bashrc`:

```bash
# Clang for bindgen
export LIBCLANG_PATH="$PREFIX/lib"

# OpenSSL for Rust builds
export OPENSSL_DIR="$PREFIX"
export OPENSSL_INCLUDE_DIR="$PREFIX/include"
export OPENSSL_LIB_DIR="$PREFIX/lib"

# PKG_CONFIG path
export PKG_CONFIG_PATH="$PREFIX/lib/pkgconfig"
```

Reload:

```bash
source ~/.bashrc
```

## Audio Dependencies

For audio capture and playback:

```bash
pkg install -y \
    pulseaudio \
    libpulseaudio
```

Note: Termux audio requires PulseAudio. The `cpal` crate will use PulseAudio backend on Termux.

### Test Audio

```bash
# Start PulseAudio
pulseaudio --start

# Test playback (if you have a test file)
paplay test.wav

# List audio devices
pactl list short sinks
pactl list short sources
```

## Project Setup

### Clone Repository

```bash
# Create projects directory
mkdir -p ~/projects
cd ~/projects

# Clone via SSH (recommended)
git clone git@github.com:your-org/lexicon-local-server.git

# Or via HTTPS
git clone https://github.com/your-org/lexicon-local-server.git

cd lexicon-local-server
```

### Initial Build Test

Test that the build environment works:

```bash
# Check cargo can parse the project
cargo check 2>&1 | head -50

# Build with minimal features first
cargo build -p lexicon-core --no-default-features --features "storage"
```

### Full Build

Once dependencies are confirmed working:

```bash
cargo build --workspace --all-features
```

Note: First build will take significant time (30-60 minutes) as it compiles all dependencies including whisper.cpp.

### Build Optimization

For faster subsequent builds, configure cargo:

```bash
mkdir -p ~/.cargo
cat >> ~/.cargo/config.toml << 'EOF'
[build]
# Use all available cores
jobs = 4

[target.aarch64-linux-android]
linker = "clang"

[profile.dev]
# Faster debug builds
opt-level = 0
debug = 1

[profile.release]
# Optimized release builds
opt-level = 3
lto = "thin"
EOF
```

Adjust `jobs` based on your device's CPU cores.

## Memory Management

Android devices have limited RAM. Configure for constrained environments:

### Swap (Optional)

Create swap file to prevent OOM during compilation:

```bash
# Create 2GB swap file
dd if=/dev/zero of=~/swapfile bs=1M count=2048
chmod 600 ~/swapfile
mkswap ~/swapfile

# Enable (requires each session, root optional)
# This may not work without root
```

### Limit Cargo Parallelism

If builds fail with OOM:

```bash
# Reduce parallel jobs
export CARGO_BUILD_JOBS=2

# Or in cargo config
echo '[build]
jobs = 2' >> ~/.cargo/config.toml
```

### Monitor Memory

```bash
# During build, watch memory usage
watch -n 1 free -h

# Or use htop
pkg install htop
htop
```

## Development Tools

### Useful Utilities

```bash
pkg install -y \
    ripgrep \
    fd \
    tree \
    jq \
    htop \
    tmux
```

### tmux Configuration

For persistent sessions:

```bash
cat > ~/.tmux.conf << 'EOF'
# Use Ctrl-a as prefix (easier on mobile)
set -g prefix C-a
unbind C-b
bind C-a send-prefix

# Mouse support
set -g mouse on

# Start windows at 1
set -g base-index 1

# Status bar
set -g status-bg black
set -g status-fg white
set -g status-left '[#S] '
set -g status-right '%H:%M'
EOF
```

Start a session:

```bash
tmux new -s dev
```

Detach: `Ctrl-a d`
Reattach: `tmux attach -t dev`

### Code Navigation

```bash
# Search code
rg "pattern" --type rust

# Find files
fd "pattern"

# Project structure
tree -L 2 -I target
```

## Verification Checklist

Run these commands to verify the environment:

```bash
# Git
git --version && git config user.name

# Rust
rustc --version && cargo --version

# Build tools
cmake --version
pkg-config --version
clang --version | head -1

# Environment variables
echo "LIBCLANG_PATH: $LIBCLANG_PATH"
echo "OPENSSL_DIR: $OPENSSL_DIR"

# Test build
cd ~/projects/lexicon-local-server
cargo check -p lexicon-core --no-default-features --features "storage"
```

Expected output: All commands succeed without errors.

## Troubleshooting

### "Unable to find libclang"

```bash
export LIBCLANG_PATH="$PREFIX/lib"
# Add to ~/.bashrc
```

### "cmake not found"

```bash
pkg install cmake
```

### Build fails with "out of memory"

- Reduce cargo jobs: `export CARGO_BUILD_JOBS=1`
- Close other apps on device
- Try building individual crates first

### "linker not found"

```bash
pkg install binutils
```

### Git SSL certificate errors

```bash
pkg install ca-certificates
git config --global http.sslVerify true
```

## Next Steps

Continue to [04-build-deployment.md](04-build-deployment.md) for automated build and deployment scripts.
