# Build and Deployment Automation

This guide covers automated build scripts, deployment workflows, and continuous integration patterns for Termux development.

## Directory Structure

Set up a standardized project layout:

```bash
mkdir -p ~/projects
mkdir -p ~/bin
mkdir -p ~/logs
mkdir -p ~/builds
```

| Directory | Purpose |
|-----------|---------|
| `~/projects/` | Source code repositories |
| `~/bin/` | Custom scripts |
| `~/logs/` | Build and run logs |
| `~/builds/` | Release artifacts |

## Build Scripts

### Main Build Script

Create `~/bin/build-lexicon`:

```bash
cat > ~/bin/build-lexicon << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash
#
# Lexicon Local Server - Termux Build Script
#
set -e

PROJECT_DIR="$HOME/projects/lexicon-local-server"
LOG_DIR="$HOME/logs"
BUILD_DIR="$HOME/builds"

# Defaults
BUILD_TYPE="debug"
FEATURES="--all-features"
VERBOSE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --release|-r)
            BUILD_TYPE="release"
            shift
            ;;
        --minimal|-m)
            FEATURES="--no-default-features --features storage,audio"
            shift
            ;;
        --verbose|-v)
            VERBOSE="-v"
            shift
            ;;
        --help|-h)
            echo "Usage: build-lexicon [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -r, --release   Build in release mode"
            echo "  -m, --minimal   Build with minimal features"
            echo "  -v, --verbose   Verbose output"
            echo "  -h, --help      Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Timestamp for logs
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
LOG_FILE="$LOG_DIR/build-$TIMESTAMP.log"

echo "=== Lexicon Build ==="
echo "Type: $BUILD_TYPE"
echo "Features: $FEATURES"
echo "Log: $LOG_FILE"
echo ""

cd "$PROJECT_DIR"

# Git status
echo "--- Git Status ---"
git rev-parse --short HEAD
git status --short

# Build command
BUILD_CMD="cargo build --workspace $FEATURES $VERBOSE"
if [[ "$BUILD_TYPE" == "release" ]]; then
    BUILD_CMD="$BUILD_CMD --release"
fi

echo ""
echo "--- Building ---"
echo "$ $BUILD_CMD"
echo ""

# Run build with logging
if $BUILD_CMD 2>&1 | tee "$LOG_FILE"; then
    echo ""
    echo "=== Build Successful ==="

    # Copy release binary
    if [[ "$BUILD_TYPE" == "release" ]]; then
        BINARY="$PROJECT_DIR/target/release/lexicon-server"
        if [[ -f "$BINARY" ]]; then
            cp "$BINARY" "$BUILD_DIR/lexicon-server-$TIMESTAMP"
            ln -sf "$BUILD_DIR/lexicon-server-$TIMESTAMP" "$BUILD_DIR/lexicon-server-latest"
            echo "Binary: $BUILD_DIR/lexicon-server-latest"
            ls -lh "$BINARY"
        fi
    fi
else
    echo ""
    echo "=== Build Failed ==="
    echo "See log: $LOG_FILE"
    exit 1
fi
EOF

chmod +x ~/bin/build-lexicon
```

### Quick Test Script

Create `~/bin/test-lexicon`:

```bash
cat > ~/bin/test-lexicon << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash
#
# Lexicon Local Server - Test Script
#
set -e

PROJECT_DIR="$HOME/projects/lexicon-local-server"
LOG_DIR="$HOME/logs"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
LOG_FILE="$LOG_DIR/test-$TIMESTAMP.log"

echo "=== Lexicon Tests ==="
echo "Log: $LOG_FILE"
echo ""

cd "$PROJECT_DIR"

# Run tests
cargo test --workspace --all-features 2>&1 | tee "$LOG_FILE"

echo ""
echo "=== Tests Complete ==="
EOF

chmod +x ~/bin/test-lexicon
```

## Git-Triggered Builds

### Post-Receive Hook (for local git server)

If hosting a bare repo on the device:

```bash
# In bare repo: hooks/post-receive
cat > hooks/post-receive << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash

WORK_DIR="$HOME/projects/lexicon-local-server"
LOG_DIR="$HOME/logs"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

echo "=== Post-Receive Hook ==="

# Update working directory
cd "$WORK_DIR"
git fetch origin
git reset --hard origin/main

# Trigger build
echo "Starting build..."
nohup ~/bin/build-lexicon --release > "$LOG_DIR/post-receive-$TIMESTAMP.log" 2>&1 &

echo "Build started in background"
EOF

chmod +x hooks/post-receive
```

### Pull and Build Script

Create `~/bin/pull-and-build`:

```bash
cat > ~/bin/pull-and-build << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash
#
# Pull latest changes and build
#
set -e

PROJECT_DIR="$HOME/projects/lexicon-local-server"
LOG_DIR="$HOME/logs"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

echo "=== Pull and Build ==="

cd "$PROJECT_DIR"

# Stash any local changes
git stash --quiet 2>/dev/null || true

# Pull latest
echo "Pulling latest..."
git pull origin main

# Show what changed
echo ""
echo "--- Changes ---"
git log --oneline -5

# Build
echo ""
~/bin/build-lexicon --release

echo ""
echo "=== Complete ==="
EOF

chmod +x ~/bin/pull-and-build
```

## Run Scripts

### Development Server

Create `~/bin/run-lexicon`:

```bash
cat > ~/bin/run-lexicon << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash
#
# Run Lexicon Server
#
PROJECT_DIR="$HOME/projects/lexicon-local-server"
LOG_DIR="$HOME/logs"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

# Parse arguments
USE_RELEASE=""
BACKGROUND=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --release|-r)
            USE_RELEASE=1
            shift
            ;;
        --background|-b)
            BACKGROUND=1
            shift
            ;;
        --help|-h)
            echo "Usage: run-lexicon [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  -r, --release    Run release build"
            echo "  -b, --background Run in background"
            echo "  -h, --help       Show this help"
            exit 0
            ;;
        *)
            shift
            ;;
    esac
done

# Select binary
if [[ -n "$USE_RELEASE" ]]; then
    BINARY="$PROJECT_DIR/target/release/lexicon-server"
else
    BINARY="$PROJECT_DIR/target/debug/lexicon-server"
fi

if [[ ! -f "$BINARY" ]]; then
    echo "Binary not found: $BINARY"
    echo "Run build-lexicon first"
    exit 1
fi

# Environment
export LEXICON_LOG=info
export LEXICON_HOST=0.0.0.0
export LEXICON_PORT=3000

echo "=== Starting Lexicon Server ==="
echo "Binary: $BINARY"
echo "Host: $LEXICON_HOST:$LEXICON_PORT"

if [[ -n "$BACKGROUND" ]]; then
    LOG_FILE="$LOG_DIR/server-$TIMESTAMP.log"
    echo "Log: $LOG_FILE"
    nohup "$BINARY" > "$LOG_FILE" 2>&1 &
    PID=$!
    echo "PID: $PID"
    echo $PID > "$LOG_DIR/server.pid"
    echo ""
    echo "Server started in background"
    echo "Stop with: kill \$(cat ~/logs/server.pid)"
else
    exec "$BINARY"
fi
EOF

chmod +x ~/bin/run-lexicon
```

### Stop Server

Create `~/bin/stop-lexicon`:

```bash
cat > ~/bin/stop-lexicon << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash
#
# Stop Lexicon Server
#
PID_FILE="$HOME/logs/server.pid"

if [[ -f "$PID_FILE" ]]; then
    PID=$(cat "$PID_FILE")
    if kill -0 "$PID" 2>/dev/null; then
        echo "Stopping server (PID: $PID)..."
        kill "$PID"
        rm "$PID_FILE"
        echo "Server stopped"
    else
        echo "Server not running (stale PID file)"
        rm "$PID_FILE"
    fi
else
    # Try to find by process name
    PID=$(pgrep -f "lexicon-server" | head -1)
    if [[ -n "$PID" ]]; then
        echo "Stopping server (PID: $PID)..."
        kill "$PID"
        echo "Server stopped"
    else
        echo "Server not running"
    fi
fi
EOF

chmod +x ~/bin/stop-lexicon
```

## Remote Deployment from Dev Machine

### Deploy Script (Windows PowerShell)

Create `scripts/deploy-termux.ps1` on your development machine:

```powershell
# deploy-termux.ps1 - Deploy to Termux device
param(
    [string]$Host = "termux",
    [switch]$BuildOnly,
    [switch]$RunAfter
)

$ErrorActionPreference = "Stop"

Write-Host "=== Deploy to Termux ===" -ForegroundColor Cyan

# Push latest commits
Write-Host "`nPushing to remote..."
git push origin main

# Trigger build on device
Write-Host "`nTriggering build on device..."
ssh $Host "cd ~/projects/lexicon-local-server && git pull && ~/bin/build-lexicon --release"

if (-not $BuildOnly) {
    Write-Host "`nDeployment complete"
}

if ($RunAfter) {
    Write-Host "`nStarting server..."
    ssh $Host "~/bin/stop-lexicon; ~/bin/run-lexicon --release --background"
    Write-Host "Server started on device"
}

Write-Host "`n=== Done ===" -ForegroundColor Green
```

### Deploy Script (Bash)

Create `scripts/deploy-termux.sh`:

```bash
#!/bin/bash
# deploy-termux.sh - Deploy to Termux device

HOST="${1:-termux}"
BUILD_ONLY="${2:-}"

echo "=== Deploy to Termux ==="

# Push latest commits
echo ""
echo "Pushing to remote..."
git push origin main

# Trigger build on device
echo ""
echo "Triggering build on device..."
ssh "$HOST" "cd ~/projects/lexicon-local-server && git pull && ~/bin/build-lexicon --release"

echo ""
echo "=== Deployment Complete ==="
```

## Continuous Build Watcher

Create `~/bin/watch-build`:

```bash
cat > ~/bin/watch-build << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash
#
# Watch for changes and rebuild
#
# Requires: pkg install inotify-tools
#
PROJECT_DIR="$HOME/projects/lexicon-local-server"

echo "=== Watching for changes ==="
echo "Directory: $PROJECT_DIR/src"
echo "Press Ctrl+C to stop"
echo ""

cd "$PROJECT_DIR"

while true; do
    inotifywait -r -e modify,create,delete src/ Cargo.toml 2>/dev/null
    echo ""
    echo "--- Change detected, rebuilding ---"
    cargo check --workspace 2>&1 | tail -20
    echo ""
done
EOF

chmod +x ~/bin/watch-build
```

Install inotify-tools:

```bash
pkg install inotify-tools
```

## Service Management

### Systemd-style Service (Using termux-services)

Install termux-services:

```bash
pkg install termux-services
```

Create service:

```bash
mkdir -p ~/.termux/sv/lexicon
cat > ~/.termux/sv/lexicon/run << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash
exec ~/bin/run-lexicon --release 2>&1
EOF

chmod +x ~/.termux/sv/lexicon/run

# Enable logging
mkdir -p ~/.termux/sv/lexicon/log
cat > ~/.termux/sv/lexicon/log/run << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash
exec svlogd -tt ~/logs/lexicon-service
EOF

chmod +x ~/.termux/sv/lexicon/log/run
```

Manage service:

```bash
# Enable service
sv-enable lexicon

# Start/stop/restart
sv start lexicon
sv stop lexicon
sv restart lexicon

# Check status
sv status lexicon
```

## Deployment Workflow Summary

### From Development Machine

```bash
# 1. Make changes locally
code .

# 2. Commit and push
git add . && git commit -m "Feature X" && git push

# 3. Deploy to device
./scripts/deploy-termux.sh

# 4. SSH in to test
ssh termux
run-lexicon --release
```

### On Device (Manual)

```bash
# Pull latest
pull-and-build

# Run server
run-lexicon --release --background

# Check logs
tail -f ~/logs/server-*.log

# Stop
stop-lexicon
```

### Automated (Watch Mode)

```bash
# Terminal 1: Watch and rebuild on changes
watch-build

# Terminal 2: Run tests continuously
while true; do test-lexicon; sleep 60; done
```

## Log Management

### Log Rotation

Create `~/bin/rotate-logs`:

```bash
cat > ~/bin/rotate-logs << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash
#
# Rotate old logs
#
LOG_DIR="$HOME/logs"
KEEP_DAYS=7

echo "Rotating logs older than $KEEP_DAYS days..."

find "$LOG_DIR" -name "*.log" -mtime +$KEEP_DAYS -delete -print

echo "Done"
EOF

chmod +x ~/bin/rotate-logs
```

Add to crontab (requires termux-job-scheduler or cronie):

```bash
pkg install cronie
crontab -e
# Add: 0 0 * * * ~/bin/rotate-logs
```

### View Recent Logs

```bash
# Latest build log
ls -t ~/logs/build-*.log | head -1 | xargs tail -50

# Latest server log
ls -t ~/logs/server-*.log | head -1 | xargs tail -f
```

## Quick Reference

| Command | Description |
|---------|-------------|
| `build-lexicon` | Build project |
| `build-lexicon -r` | Release build |
| `test-lexicon` | Run tests |
| `run-lexicon` | Start server |
| `run-lexicon -r -b` | Start release in background |
| `stop-lexicon` | Stop server |
| `pull-and-build` | Git pull + build |
| `watch-build` | Rebuild on file changes |
| `rotate-logs` | Clean old logs |

## Troubleshooting

### Build hangs

- Check memory: `free -h`
- Reduce parallelism: Edit `~/.cargo/config.toml`, set `jobs = 1`
- Kill and retry: `pkill cargo; build-lexicon`

### Server won't start

- Check port: `ss -tlnp | grep 3000`
- Check logs: `cat ~/logs/server-*.log | tail -50`
- Kill orphan: `pkill -f lexicon-server`

### SSH disconnects during build

- Use tmux: `tmux new -s build` then run build
- Increase timeouts in SSH config
- Enable wake lock: `termux-wake-lock`

### Disk space issues

```bash
# Check space
df -h ~

# Clean cargo cache
cargo clean
rm -rf ~/.cargo/registry/cache/*

# Clean old builds
rm -rf ~/builds/*
```
