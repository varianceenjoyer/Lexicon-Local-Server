# WSL Audio Setup for TUI Testing

This guide explains how to configure audio forwarding from Windows to WSL2 for testing the Linux build of Lexicon's TUI with live microphone input.

## Overview

WSL2 does not provide native access to Windows audio capture devices. To test the TUI with live audio input, we forward audio from Windows to WSL using PulseAudio over the network.

```
┌─────────────┐     TCP:4713     ┌─────────────┐     ┌─────────────┐
│   Windows   │ ───────────────► │    WSL2     │ ──► │ Lexicon TUI │
│  Microphone │                  │ PulseAudio  │     │             │
└─────────────┘                  └─────────────┘     └─────────────┘
```

## Prerequisites

- Windows 10 (build 19041+) or Windows 11
- WSL2 with Ubuntu 22.04 or later
- Administrator access on Windows

## Method 1: WSLg (Recommended for Windows 11)

Windows 11 includes WSLg which provides audio support via a PulseAudio socket. This is the simplest method if you have Windows 11.

### Step 1: Verify WSLg is Enabled

```bash
# In WSL, check for the PulseAudio socket
ls -la /mnt/wslg/PulseServer
```

If the socket exists, WSLg audio is available.

### Step 2: Configure Environment

```bash
# Add to ~/.bashrc or ~/.zshrc
export PULSE_SERVER=unix:/mnt/wslg/PulseServer
```

### Step 3: Verify Audio

```bash
# List audio sources
pactl list sources short

# Test recording (5 seconds)
arecord -d 5 -f cd test.wav && aplay test.wav
```

### Step 4: Test the TUI

```bash
cd /path/to/Lexicon-Local-Server
./target/release/lexicon tui --model models/ggml-base.en.bin
```

The level meter should respond to your voice.

**Note:** WSLg audio capture may not work in all configurations. If Method 1 fails, use Method 2.

---

## Method 2: PulseAudio TCP Server (Windows 10/11)

This method runs a PulseAudio server on Windows that accepts network connections from WSL.

### Windows-Side Setup

#### Step 1: Download PulseAudio for Windows

Download from: https://www.freedesktop.org/wiki/Software/PulseAudio/Ports/Windows/Support/

Or use a pre-built package:
```powershell
# Using Chocolatey (run as Administrator)
choco install pulseaudio
```

#### Step 2: Configure PulseAudio

Create/edit `%APPDATA%\PulseAudio\etc\pulse\default.pa`:

```
# Load Windows audio modules
load-module module-waveout sink_name=output source_name=input
load-module module-native-protocol-tcp auth-ip-acl=127.0.0.1;172.16.0.0/12 auth-anonymous=1
```

Create/edit `%APPDATA%\PulseAudio\etc\pulse\daemon.conf`:

```
exit-idle-time = -1
```

#### Step 3: Configure Windows Firewall

```powershell
# Run as Administrator
New-NetFirewallRule -DisplayName "PulseAudio" -Direction Inbound -LocalPort 4713 -Protocol TCP -Action Allow
```

#### Step 4: Start PulseAudio

```powershell
pulseaudio.exe --exit-idle-time=-1
```

To run at startup, create a scheduled task or shortcut in the Startup folder.

### WSL-Side Setup

#### Step 1: Install PulseAudio Packages

```bash
sudo apt update
sudo apt install pulseaudio pulseaudio-utils alsa-utils
```

#### Step 2: Get Windows Host IP

```bash
# The Windows host IP in WSL2 is the default gateway
export WIN_HOST=$(ip route show default | awk '{print $3}')
echo "Windows host IP: $WIN_HOST"
```

#### Step 3: Configure PULSE_SERVER

```bash
# Add to ~/.bashrc or ~/.zshrc
export PULSE_SERVER=tcp:$(ip route show default | awk '{print $3}'):4713
```

Reload your shell:
```bash
source ~/.bashrc
```

#### Step 4: Verify Connection

```bash
# Check PulseAudio server info
pactl info

# Expected output should show:
# Server String: tcp:172.x.x.x:4713
# Server Name: pulseaudio
```

#### Step 5: List and Select Audio Source

```bash
# List available sources
pactl list sources short

# Set the default source (use the source name from above)
pactl set-default-source <source-name>
```

### Testing

#### Test Audio Capture

```bash
# Record 5 seconds of audio
arecord -d 5 -f cd test.wav

# Play it back
aplay test.wav
```

#### Test the TUI

```bash
cd /path/to/Lexicon-Local-Server
./target/release/lexicon tui --model models/ggml-base.en.bin -v
```

The `-v` flag enables verbose logging to help diagnose issues.

---

## Troubleshooting

### Connection Refused

**Symptom:** `pactl info` fails with "Connection refused"

**Solutions:**
1. Verify PulseAudio is running on Windows:
   ```powershell
   Get-Process pulseaudio
   ```

2. Check firewall rules:
   ```powershell
   Get-NetFirewallRule -DisplayName "PulseAudio"
   ```

3. Verify the IP address:
   ```bash
   # In WSL
   echo $PULSE_SERVER
   ping -c 1 $(ip route show default | awk '{print $3}')
   ```

### No Audio Sources

**Symptom:** `pactl list sources` shows no sources or only "Monitor" sources

**Solutions:**
1. Check Windows audio settings - ensure microphone is enabled
2. Verify PulseAudio loaded the waveout module:
   ```powershell
   pulseaudio.exe --dump-conf
   ```

### High Latency

**Symptom:** Audio has noticeable delay

**Solutions:**
1. Reduce buffer size in PulseAudio:
   ```
   # In default.pa
   load-module module-waveout sink_name=output source_name=input buffer_size=1024
   ```

2. Lower the fragment size in daemon.conf:
   ```
   default-fragment-size-msec = 5
   ```

### Permission Denied

**Symptom:** `pactl` commands fail with permission errors

**Solutions:**
1. Ensure `auth-anonymous=1` is set in Windows PulseAudio config
2. Alternatively, set up cookie-based authentication:
   ```bash
   # Copy cookie from Windows
   cp /mnt/c/Users/<username>/.config/pulse/cookie ~/.config/pulse/
   ```

### TUI Shows No Level Meter Activity

**Symptom:** TUI runs but level meter doesn't respond to audio

**Solutions:**
1. Verify audio is being captured:
   ```bash
   arecord -d 2 -f cd - | aplay
   ```

2. Check the TUI is using the correct device:
   ```bash
   lexicon tui --device "$(pactl get-default-source)" -v
   ```

3. Check for errors in verbose output

---

## Security Considerations

The configurations above allow anonymous connections from the local network. For a more secure setup:

1. **Restrict IP range:** Modify `auth-ip-acl` to only allow your WSL IP range
2. **Use cookie authentication:** Remove `auth-anonymous=1` and share the cookie file
3. **Don't expose to external network:** Ensure PulseAudio only binds to localhost or the WSL virtual network

---

## Quick Reference

### Start Audio Tunnel

```bash
# Windows (PowerShell)
pulseaudio.exe --exit-idle-time=-1

# WSL
export PULSE_SERVER=tcp:$(ip route show default | awk '{print $3}'):4713
```

### Test Audio

```bash
pactl info                    # Verify connection
pactl list sources short      # List audio sources
arecord -d 5 test.wav        # Record test
```

### Run TUI

```bash
./target/release/lexicon tui --model models/ggml-base.en.bin -v
```

---

## See Also

- [TUI User Guide](../../MANUAL.md#lexicon-tui)
- [Building on Linux](../../BUILD.md#linux-ubuntudebian)
- [WSL2 Documentation](https://docs.microsoft.com/en-us/windows/wsl/)
