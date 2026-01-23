# WSL Audio Setup for TUI Testing

This guide explains how to configure audio forwarding from Windows to WSL2 for testing the Lexicon TUI with live microphone input.

## Background

WSL2 does not provide native access to Windows audio capture devices. To test the Linux build of the TUI with live microphone input, we need to forward audio from Windows to the WSL environment using PulseAudio over the network.

## Prerequisites

- Windows 10/11 with WSL2 installed
- Ubuntu or other Linux distribution in WSL2
- An audio input device (microphone) on Windows

## Quick Start (WSLg)

If you're using a recent version of Windows 11 with WSLg (Windows Subsystem for Linux GUI), audio may already work out of the box. Test first:

```bash
# In WSL
pactl info
```

If you see a valid PulseAudio server connection, try running the TUI directly:

```bash
lexicon tui --model ~/.local/share/lexicon/models/ggml-base.en.bin
```

If the level meter responds to your voice, you're done! If not, continue with the manual setup below.

---

## Method 1: Using WSLg (Recommended for Windows 11)

WSLg provides a PulseAudio server automatically. However, it may only support audio playback by default. To enable capture:

### Step 1: Check WSLg Audio Server

```bash
# In WSL
echo $PULSE_SERVER
# Should show something like: /mnt/wslg/PulseServer
```

### Step 2: Verify Available Sources

```bash
pactl list sources short
```

Look for sources with "input" or "monitor" in the name.

### Step 3: Test Audio Capture

```bash
# Record 5 seconds of audio
arecord -d 5 -f cd test.wav

# Play it back
aplay test.wav
```

If capture works, the TUI should work as well.

---

## Method 2: PulseAudio Network Setup (Windows 10 / Manual)

For systems without WSLg or where the automatic setup doesn't provide capture, set up PulseAudio manually.

### Windows Side Setup

#### Step 1: Install PulseAudio for Windows

Download PulseAudio for Windows from:
- https://www.freedesktop.org/wiki/Software/PulseAudio/Ports/Windows/Support/

Or use a package manager like Chocolatey:
```powershell
choco install pulseaudio
```

#### Step 2: Configure PulseAudio Daemon

Edit `%APPDATA%\pulse\daemon.conf` (create if it doesn't exist):

```ini
exit-idle-time = -1
load-default-script-file = yes
```

#### Step 3: Configure PulseAudio Modules

Edit `%APPDATA%\pulse\default.pa`:

```ini
# Load Windows audio modules
load-module module-waveout sink_name=output source_name=input
load-module module-native-protocol-tcp auth-anonymous=1

# Set default source/sink
set-default-source input
set-default-sink output
```

#### Step 4: Configure Windows Firewall

Open PowerShell as Administrator:

```powershell
# Allow PulseAudio through firewall (port 4713)
New-NetFirewallRule -DisplayName "PulseAudio" -Direction Inbound -Protocol TCP -LocalPort 4713 -Action Allow
```

#### Step 5: Start PulseAudio

```cmd
pulseaudio.exe -D
```

To start automatically, add a shortcut to `shell:startup`.

### WSL Side Setup

#### Step 1: Install PulseAudio Client

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install pulseaudio-utils

# Fedora
sudo dnf install pulseaudio-utils
```

#### Step 2: Get Windows IP Address

```bash
# Get the Windows host IP from WSL
export WIN_IP=$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}')
echo "Windows IP: $WIN_IP"
```

#### Step 3: Configure PULSE_SERVER

Add to your `~/.bashrc` or `~/.zshrc`:

```bash
# PulseAudio server on Windows host
export PULSE_SERVER=tcp:$(cat /etc/resolv.conf | grep nameserver | awk '{print $2}'):4713
```

Reload your shell:
```bash
source ~/.bashrc
```

#### Step 4: Verify Connection

```bash
# Check PulseAudio server info
pactl info

# List available audio sources
pactl list sources short
```

You should see the Windows audio devices listed.

---

## Testing the TUI

Once audio forwarding is configured:

### Step 1: Verify Audio Capture Works

```bash
# Test recording (speak into microphone for 5 seconds)
arecord -d 5 -f cd test.wav
aplay test.wav  # Should hear your recording
```

### Step 2: Run the TUI

```bash
# Download a model if you haven't already
lexicon models download base.en

# Run the TUI
lexicon tui --model ~/.local/share/lexicon/models/ggml-base.en.bin
```

### Step 3: Verify Level Meter

- The level meter should respond when you speak
- If not, try selecting a different audio device with Tab

---

## Troubleshooting

### "Connection refused" Error

1. Verify PulseAudio is running on Windows:
   ```cmd
   tasklist | findstr pulse
   ```

2. Check Windows Firewall isn't blocking port 4713

3. Verify the IP address is correct:
   ```bash
   ping $WIN_IP
   ```

### No Audio Sources Visible

1. Check Windows audio devices are enabled:
   - Right-click speaker icon → Sound settings
   - Verify microphone is listed and enabled

2. Restart PulseAudio on Windows:
   ```cmd
   pulseaudio.exe --kill
   pulseaudio.exe -D
   ```

### High Latency / Audio Dropouts

Edit `~/.config/pulse/daemon.conf` in WSL:

```ini
default-fragments = 4
default-fragment-size-msec = 25
```

Then restart PulseAudio client:
```bash
pulseaudio --kill
pulseaudio --start
```

### Permission Denied

If using cookie-based authentication instead of anonymous:

```bash
# Copy cookie from Windows
cp /mnt/c/Users/$USER/.config/pulse/cookie ~/.config/pulse/
```

---

## Security Considerations

The configuration above uses `auth-anonymous=1` which allows any local application to connect to PulseAudio. For better security:

1. Use cookie-based authentication instead
2. Restrict listening to localhost only:
   ```ini
   load-module module-native-protocol-tcp auth-ip-acl=127.0.0.1;172.16.0.0/12 auth-anonymous=1
   ```

3. Use SSH tunneling for remote access

---

## Alternative: Virtual Audio Cable

For more complex setups, consider using a virtual audio cable:

1. Install VB-Cable (https://vb-audio.com/Cable/)
2. Route microphone to VB-Cable
3. Use VB-Cable as the PulseAudio source

This provides more flexibility for routing and mixing audio.

---

## Useful Commands

```bash
# List all PulseAudio sources (inputs)
pactl list sources

# List all PulseAudio sinks (outputs)
pactl list sinks

# Set default source
pactl set-default-source <source_name>

# Test microphone with real-time playback
parec | paplay

# Monitor audio levels
pactl subscribe
```

---

*Last updated: 2026-01-23*
