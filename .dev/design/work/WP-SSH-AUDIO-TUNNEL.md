---
id: WP-SSH-AUDIO-TUNNEL
type: work-plan
status: complete
phase: beta
created: 2026-01-23
updated: 2026-01-23
author: design-lead
consumers: [developer, qa]
depends-on: []
blocks: []
priority: medium
tags: [documentation, wsl, linux, audio, testing]
---

# WP-SSH-AUDIO-TUNNEL: SSH Audio Tunnel Documentation for WSL Testing

> **Phase**: Beta | **Priority**: MEDIUM

## Summary

Create documentation for configuring an SSH tunnel to forward audio from Windows to WSL (Windows Subsystem for Linux). This enables testing the Linux build of the TUI with live audio input, since WSL doesn't have native access to Windows audio devices.

---

## Background

### Problem
- WSL2 does not provide native access to Windows audio capture devices
- The Linux build of the TUI needs to be tested with live microphone input
- The only viable method is to capture audio on Windows and stream it to the Linux environment

### Solution Overview
Configure PulseAudio (or PipeWire) on the Windows side to accept network connections, then configure the WSL environment to use the Windows audio server as its audio source.

---

## Scope

### Included
- Document PulseAudio-over-network setup for Windows → WSL audio
- Alternative: Document PipeWire configuration if applicable
- Document WSL2 configuration steps
- Provide verification/testing steps
- Include troubleshooting section

### Excluded
- Native audio passthrough (not currently possible in WSL2)
- Docker audio configuration
- Remote machine audio forwarding (different network)

---

## Documentation Outline

### 1. Prerequisites
- Windows 10/11 with WSL2 installed
- Ubuntu or other Linux distro in WSL2
- PulseAudio or PipeWire installed in WSL

### 2. Windows-Side Configuration

#### Option A: PulseAudio for Windows
```
1. Install PulseAudio for Windows
2. Configure daemon.conf to allow network access
3. Configure default.pa to load tcp module
4. Open firewall for PulseAudio port (4713)
5. Start PulseAudio daemon
```

#### Option B: Use Windows' Built-in Audio with WSLg
```
Note: WSLg (Windows Subsystem for Linux GUI) includes audio support
via PulseAudio socket. Verify if this is sufficient for capture.
```

### 3. WSL-Side Configuration
```
1. Install PulseAudio client packages
2. Configure PULSE_SERVER environment variable
3. Verify audio routing with pactl/pacmd
4. Test capture with arecord or similar
```

### 4. Testing the TUI
```bash
# In WSL
export PULSE_SERVER=tcp:$(hostname).local:4713
./target/release/lexicon tui --model models/ggml-base.en.bin
```

### 5. Troubleshooting
- Connection refused: firewall rules
- No audio: source selection
- Latency issues: buffer configuration
- Permission denied: authentication setup

---

## Documentation Location

Create documentation at:
```
docs/testing/wsl-audio-setup.md
```

This aligns with the existing docs structure and the testing focus of this configuration.

---

## Content Requirements

### Must Include
- Step-by-step instructions with exact commands
- Configuration file examples (copy-paste ready)
- Verification commands at each step
- Common error messages and solutions
- Security considerations (network audio exposure)

### Nice to Have
- Diagram of audio flow (Windows → Network → WSL → TUI)
- Alternative methods comparison
- Performance tuning tips
- Script to automate setup

---

## Acceptance Criteria

- [x] Documentation created at `docs/testing/wsl-audio-setup.md`
- [x] Windows PulseAudio server configuration documented
- [x] WSL client configuration documented
- [x] All commands tested and verified working
- [x] Verification steps confirm audio is captured
- [x] TUI successfully receives audio via tunnel
- [x] Troubleshooting section covers common issues

---

## Research Required

Before writing documentation, verify:

1. **WSLg Audio Support**
   - Does WSLg provide audio capture (not just playback)?
   - If yes, document the simpler WSLg approach first

2. **PulseAudio vs PipeWire**
   - Which is more commonly available in modern WSL distros?
   - Document the more common option first

3. **Security Implications**
   - Is cookie-based auth required?
   - Should audio be restricted to localhost only?

---

## Files to Create

| File | Description |
|------|-------------|
| `docs/testing/wsl-audio-setup.md` | Main documentation file |
| `docs/testing/scripts/wsl-audio-setup.sh` | Optional setup script for WSL side |

---

## Test Plan

| Test | Procedure | Pass Criteria |
|------|-----------|---------------|
| PulseAudio connection | `pactl info` in WSL | Shows Windows server |
| Audio capture test | `arecord -d 5 test.wav` in WSL | Captures Windows mic input |
| TUI test | `lexicon tui` in WSL | Level meter responds to voice |
| Transcription test | Full transcription in WSL TUI | Text appears correctly |

---

## Dependencies

- WSL2 installed and configured
- Linux build of Lexicon compiles (WP-TUI complete)

---

## Changelog

| Date | Update |
|------|--------|
| 2026-01-23 | Work plan created for WSL audio testing documentation |
| 2026-01-23 | Documentation complete: docs/testing/wsl-audio-setup.md |
