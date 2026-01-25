# SSH Configuration

This guide covers setting up OpenSSH server on Termux for remote access from your development machine.

## Installing OpenSSH

```bash
pkg install -y openssh
```

This installs both the SSH client and server.

## Setting Up the "dev" User

Termux runs as a single user, but we configure it as a "dev" environment:

### Set Password

Required for initial SSH authentication:

```bash
passwd
```

Enter a strong password when prompted.

### Verify Username

Termux uses a generated username. Find yours:

```bash
whoami
# Typically outputs: u0_a<number>
```

Note this username for SSH connections.

## SSH Server Configuration

### Generate Host Keys

```bash
# Generate host keys if not present
ssh-keygen -A
```

### Configure sshd

Create/edit the SSH daemon configuration:

```bash
nano $PREFIX/etc/ssh/sshd_config
```

Recommended configuration:

```sshconfig
# Termux SSH Server Configuration

# Port (must be > 1024 without root)
Port 8022

# Protocol
Protocol 2

# Host keys
HostKey /data/data/com.termux/files/usr/etc/ssh/ssh_host_rsa_key
HostKey /data/data/com.termux/files/usr/etc/ssh/ssh_host_ed25519_key

# Authentication
PermitRootLogin no
PubkeyAuthentication yes
AuthorizedKeysFile .ssh/authorized_keys
PasswordAuthentication yes

# Security
MaxAuthTries 3
MaxSessions 5
ClientAliveInterval 60
ClientAliveCountMax 3

# Subsystems
Subsystem sftp /data/data/com.termux/files/usr/libexec/sftp-server

# Logging
PrintMotd yes
```

## Starting the SSH Server

### Manual Start

```bash
sshd
```

Verify it's running:

```bash
pgrep sshd
# or
ps aux | grep sshd
```

### Get Device IP Address

```bash
# WiFi IP
ip addr show wlan0 | grep 'inet ' | awk '{print $2}' | cut -d/ -f1

# All interfaces
ip addr | grep 'inet ' | awk '{print $2}'
```

### Stop SSH Server

```bash
pkill sshd
```

## Key-Based Authentication

Password authentication works but key-based is more secure and convenient.

### On Your Development Machine

#### Generate SSH Key (if needed)

**Windows PowerShell:**
```powershell
ssh-keygen -t ed25519 -C "dev@termux"
```

**Linux/macOS:**
```bash
ssh-keygen -t ed25519 -C "dev@termux"
```

Default location: `~/.ssh/id_ed25519` (private) and `~/.ssh/id_ed25519.pub` (public)

#### Copy Public Key to Device

**Option 1: ssh-copy-id (Linux/macOS)**

```bash
ssh-copy-id -p 8022 -i ~/.ssh/id_ed25519.pub u0_a123@<device-ip>
```

**Option 2: Manual Copy (Windows/any)**

1. Display your public key:
   ```powershell
   cat ~/.ssh/id_ed25519.pub
   ```

2. On Termux, add to authorized_keys:
   ```bash
   mkdir -p ~/.ssh
   chmod 700 ~/.ssh
   nano ~/.ssh/authorized_keys
   # Paste your public key, save
   chmod 600 ~/.ssh/authorized_keys
   ```

### Disable Password Authentication (Optional)

Once key auth works, disable password login for security:

```bash
nano $PREFIX/etc/ssh/sshd_config
```

Change:
```sshconfig
PasswordAuthentication no
```

Restart sshd:
```bash
pkill sshd && sshd
```

## SSH Client Configuration

Configure your development machine for easy access.

### Windows/Linux/macOS SSH Config

Edit `~/.ssh/config`:

```sshconfig
Host termux
    HostName 192.168.1.100  # Replace with device IP
    Port 8022
    User u0_a123            # Replace with Termux username
    IdentityFile ~/.ssh/id_ed25519
    ServerAliveInterval 60
    ServerAliveCountMax 3

Host termux-local
    HostName 192.168.1.100
    Port 8022
    User u0_a123
    IdentityFile ~/.ssh/id_ed25519
    LocalForward 3000 localhost:3000  # Forward server port
```

Now connect with:
```bash
ssh termux
```

## Auto-Start SSH on Boot

### Using Termux:Boot (Recommended)

1. Install Termux:Boot from F-Droid
2. Create boot script:
   ```bash
   mkdir -p ~/.termux/boot
   cat > ~/.termux/boot/start-sshd.sh << 'EOF'
   #!/data/data/com.termux/files/usr/bin/bash
   termux-wake-lock
   sshd
   EOF
   chmod +x ~/.termux/boot/start-sshd.sh
   ```
3. Open Termux:Boot app once to enable
4. Reboot device to test

### Using .bashrc (Fallback)

Add to `~/.bashrc`:

```bash
# Start sshd if not running
if ! pgrep -x sshd > /dev/null; then
    sshd
    echo "SSH server started on port 8022"
fi
```

Note: This only starts when you open Termux manually.

## Networking Considerations

### Static IP / DHCP Reservation

For reliable access, assign a static IP to your Android device:

1. Router admin panel → DHCP reservations
2. Add device MAC address with fixed IP

Or configure static IP on device:
1. Settings → WiFi → your network → Advanced
2. IP settings: Static
3. Set IP, gateway, DNS

### Firewall/Port Forwarding

If connecting from outside local network:
- Forward port 8022 on router to device IP
- Consider using a non-standard port for security
- VPN is recommended over direct exposure

### mDNS/Bonjour (Optional)

For automatic discovery:

```bash
pkg install avahi
avahi-daemon --no-drop-root -s &
```

Connect using hostname:
```bash
ssh -p 8022 termux.local
```

## Connection Testing

### From Development Machine

```bash
# Test connection
ssh -p 8022 u0_a123@<device-ip>

# Verbose mode for debugging
ssh -v -p 8022 u0_a123@<device-ip>

# Test SFTP
sftp -P 8022 u0_a123@<device-ip>
```

### Expected Output

```
$ ssh termux
Welcome to Termux!

Termux ready - aarch64 - Sat Jan 25 10:30:00 UTC 2026
u0_a123@termux:~$
```

## Troubleshooting

### "Connection refused"

- Verify sshd is running: `pgrep sshd`
- Check port: `ss -tlnp | grep 8022`
- Start sshd: `sshd`

### "Permission denied (publickey)"

- Check key permissions: `ls -la ~/.ssh/`
- Verify authorized_keys content
- Try password auth: `ssh -o PubkeyAuthentication=no ...`

### "Host key verification failed"

Device IP changed or reinstalled:
```bash
ssh-keygen -R "[<device-ip>]:8022"
```

### Connection drops/timeouts

- Enable wake lock: `termux-wake-lock`
- Disable battery optimization for Termux
- Check WiFi sleep settings
- Add to sshd_config:
  ```
  ClientAliveInterval 30
  ClientAliveCountMax 5
  ```

### Slow connection

- Disable DNS lookup in sshd_config:
  ```
  UseDNS no
  ```
- Check for network congestion

## Security Best Practices

1. **Use key authentication** - Disable password auth after setup
2. **Non-default port** - Consider changing from 8022
3. **Fail2ban alternative** - Monitor `/data/data/com.termux/files/usr/var/log/` for brute force
4. **Keep updated** - `pkg upgrade` regularly
5. **Limit access** - Use `AllowUsers` in sshd_config if needed

## Next Steps

Continue to [03-dev-environment.md](03-dev-environment.md) to set up the development tools.
