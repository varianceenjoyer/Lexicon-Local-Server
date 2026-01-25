# Lexicon TUI - Android Cross-Compilation Script (Windows)
#
# Cross-compiles the lexicon CLI binary for Android (aarch64-linux-android).
# The resulting binary can run in Termux on Android devices.
#
# Usage:
#   .\scripts\android\build-android-tui.ps1              # Debug build
#   .\scripts\android\build-android-tui.ps1 -Release     # Release build
#   .\scripts\android\build-android-tui.ps1 -Push        # Build and push to device via adb
#   .\scripts\android\build-android-tui.ps1 -Features "stt,storage"  # Specific features
#
# Prerequisites:
#   - Rust with aarch64-linux-android target: rustup target add aarch64-linux-android
#   - Android NDK r26+ (set ANDROID_NDK_HOME or install via Android Studio)
#   - CMake (for whisper-rs)
#   - LLVM/Clang (for bindgen)
#   - ADB (optional, for -Push)
#
# Environment Variables:
#   ANDROID_NDK_HOME - Path to Android NDK (required)
#   ANDROID_API_LEVEL - API level (default: 26 for Android 8.0+)

param(
    [switch]$Release,
    [switch]$Push,
    [string]$Features = "",
    [string]$Target = "arm64",
    [switch]$Help
)

$ErrorActionPreference = "Stop"

# Configuration
$DefaultApiLevel = 26
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent (Split-Path -Parent $ScriptDir)

# Target mapping
$TargetMap = @{
    "arm64"  = "aarch64-linux-android"
    "x86_64" = "x86_64-linux-android"
    "arm32"  = "armv7-linux-androideabi"
    "x86"    = "i686-linux-android"
}

$AbiMap = @{
    "arm64"  = "arm64-v8a"
    "x86_64" = "x86_64"
    "arm32"  = "armeabi-v7a"
    "x86"    = "x86"
}

function Write-Info {
    param([string]$Message)
    Write-Host "[INFO] " -ForegroundColor Green -NoNewline
    Write-Host $Message
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[WARN] " -ForegroundColor Yellow -NoNewline
    Write-Host $Message
}

function Write-Error-Custom {
    param([string]$Message)
    Write-Host "[ERROR] " -ForegroundColor Red -NoNewline
    Write-Host $Message
    exit 1
}

function Write-Step {
    param([string]$Message)
    Write-Host ""
    Write-Host "=== $Message ===" -ForegroundColor Cyan
}

function Show-Help {
    Write-Host @"
Lexicon TUI - Android Cross-Compilation Script

USAGE:
    .\build-android-tui.ps1 [OPTIONS]

OPTIONS:
    -Release        Build in release mode (optimized, stripped)
    -Push           Push binary to connected Android device via adb
    -Features       Comma-separated features (default: stt,storage,audio)
    -Target         Target architecture: arm64 (default), x86_64, arm32, x86
    -Help           Show this help message

EXAMPLES:
    # Debug build for ARM64
    .\build-android-tui.ps1

    # Release build
    .\build-android-tui.ps1 -Release

    # Build and push to device
    .\build-android-tui.ps1 -Release -Push

    # Build with specific features
    .\build-android-tui.ps1 -Features "storage"

PREREQUISITES:
    1. Install Android NDK r26+ (via Android Studio SDK Manager or standalone)
    2. Set ANDROID_NDK_HOME environment variable
    3. Install Rust target: rustup target add aarch64-linux-android
    4. Install CMake and LLVM (for native dependencies)

DEVICE SETUP (Termux):
    # After pushing the binary:
    chmod +x ~/lexicon
    ./lexicon --help
"@
    exit 0
}

function Find-AndroidNdk {
    # Check environment variable first
    if ($env:ANDROID_NDK_HOME -and (Test-Path $env:ANDROID_NDK_HOME)) {
        return $env:ANDROID_NDK_HOME
    }

    # Common installation paths on Windows
    $searchPaths = @(
        "$env:LOCALAPPDATA\Android\Sdk\ndk\*",
        "$env:USERPROFILE\Android\Sdk\ndk\*",
        "$env:ANDROID_HOME\ndk\*",
        "C:\Android\ndk\*",
        "C:\android-ndk-*"
    )

    foreach ($pattern in $searchPaths) {
        $found = Get-ChildItem -Path $pattern -Directory -ErrorAction SilentlyContinue |
                 Sort-Object Name -Descending |
                 Select-Object -First 1
        if ($found) {
            return $found.FullName
        }
    }

    return $null
}

function Get-NdkToolchain {
    param([string]$NdkPath, [string]$RustTarget, [int]$ApiLevel)

    # Determine host platform
    $hostPlatform = "windows-x86_64"

    $toolchainBase = Join-Path $NdkPath "toolchains\llvm\prebuilt\$hostPlatform\bin"

    if (-not (Test-Path $toolchainBase)) {
        Write-Error-Custom "NDK toolchain not found at: $toolchainBase"
    }

    # Map Rust target to clang prefix
    $clangPrefix = switch ($RustTarget) {
        "aarch64-linux-android" { "aarch64-linux-android" }
        "x86_64-linux-android" { "x86_64-linux-android" }
        "armv7-linux-androideabi" { "armv7a-linux-androideabi" }
        "i686-linux-android" { "i686-linux-android" }
        default { Write-Error-Custom "Unknown target: $RustTarget" }
    }

    # Windows uses .cmd wrapper scripts
    $clang = Join-Path $toolchainBase "$clangPrefix$ApiLevel-clang.cmd"
    $clangxx = Join-Path $toolchainBase "$clangPrefix$ApiLevel-clang++.cmd"
    $ar = Join-Path $toolchainBase "llvm-ar.exe"

    if (-not (Test-Path $clang)) {
        Write-Error-Custom "Clang not found: $clang`nEnsure NDK r26+ is installed and API level $ApiLevel is available"
    }

    return @{
        CC = $clang
        CXX = $clangxx
        AR = $ar
        ToolchainBase = $toolchainBase
    }
}

function Test-RustTarget {
    param([string]$Target)

    $installed = rustup target list --installed 2>$null
    if ($installed -notcontains $Target) {
        Write-Info "Installing Rust target: $Target"
        rustup target add $Target
        if ($LASTEXITCODE -ne 0) {
            Write-Error-Custom "Failed to install Rust target: $Target"
        }
    }
}

function Build-AndroidBinary {
    param(
        [string]$NdkPath,
        [string]$RustTarget,
        [int]$ApiLevel,
        [bool]$IsRelease,
        [string]$Features
    )

    Write-Step "Building for $RustTarget"

    # Get toolchain paths
    $toolchain = Get-NdkToolchain -NdkPath $NdkPath -RustTarget $RustTarget -ApiLevel $ApiLevel

    # Set environment variables for cross-compilation
    $env:CC = $toolchain.CC
    $env:CXX = $toolchain.CXX
    $env:AR = $toolchain.AR

    # For Cargo to use the right linker
    $linkerEnvVar = "CARGO_TARGET_$($RustTarget.ToUpper().Replace('-', '_'))_LINKER"
    [Environment]::SetEnvironmentVariable($linkerEnvVar, $toolchain.CC, "Process")

    # CMake toolchain for whisper-rs
    $env:CMAKE_TOOLCHAIN_FILE = Join-Path $NdkPath "build\cmake\android.toolchain.cmake"
    $env:ANDROID_ABI = $AbiMap[$Target]
    $env:ANDROID_PLATFORM = "android-$ApiLevel"

    # Skip bindgen regeneration (use pre-generated bindings)
    $env:WHISPER_DONT_GENERATE_BINDINGS = "1"

    # whisper.cpp build options
    $env:WHISPER_BUILD_TESTS = "OFF"
    $env:WHISPER_BUILD_EXAMPLES = "OFF"

    Write-Info "CC: $($toolchain.CC)"
    Write-Info "Target: $RustTarget"
    Write-Info "API Level: $ApiLevel"
    Write-Info "CMake Toolchain: $($env:CMAKE_TOOLCHAIN_FILE)"

    # Build command
    $cargoArgs = @(
        "build",
        "-p", "lexicon-cli",
        "--target", $RustTarget
    )

    if ($IsRelease) {
        $cargoArgs += "--release"
    }

    # Features: use minimal set for Android
    if ($Features) {
        $cargoArgs += "--no-default-features"
        $cargoArgs += "--features"
        $cargoArgs += $Features
    }

    Write-Host ""
    Write-Info "Running: cargo $($cargoArgs -join ' ')"
    Write-Host ""

    Set-Location $ProjectDir
    & cargo @cargoArgs

    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Build failed"
    }

    # Output location
    $buildType = if ($IsRelease) { "release" } else { "debug" }
    $binaryPath = Join-Path $ProjectDir "target\$RustTarget\$buildType\lexicon"

    # On Windows, the binary might not have .exe but it's an ELF binary
    if (-not (Test-Path $binaryPath)) {
        # Try with .exe extension (shouldn't exist for Android target)
        $binaryPathExe = "$binaryPath.exe"
        if (Test-Path $binaryPathExe) {
            $binaryPath = $binaryPathExe
        } else {
            Write-Error-Custom "Binary not found at: $binaryPath"
        }
    }

    Write-Host ""
    Write-Info "Build successful!"
    Write-Info "Binary: $binaryPath"

    # Show file info
    $fileInfo = Get-Item $binaryPath
    Write-Info "Size: $([math]::Round($fileInfo.Length / 1MB, 2)) MB"

    return $binaryPath
}

function Push-ToDevice {
    param([string]$BinaryPath)

    Write-Step "Pushing to Android Device"

    # Check if adb is available
    if (-not (Get-Command adb -ErrorAction SilentlyContinue)) {
        Write-Warn "adb not found in PATH"
        Write-Warn "Add Android SDK platform-tools to PATH or push manually:"
        Write-Warn "  adb push `"$BinaryPath`" /data/local/tmp/lexicon"
        return
    }

    # Check for connected device
    $devices = adb devices 2>$null | Select-String "device$"
    if (-not $devices) {
        Write-Warn "No Android device connected"
        Write-Warn "Connect a device with USB debugging enabled and try again"
        Write-Warn "Manual push: adb push `"$BinaryPath`" /data/local/tmp/lexicon"
        return
    }

    Write-Info "Pushing binary to device..."

    # Push to Termux-accessible location
    # /data/local/tmp is accessible without root
    $remotePath = "/data/local/tmp/lexicon"

    adb push $BinaryPath $remotePath
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Failed to push binary to device"
    }

    # Make executable
    adb shell chmod +x $remotePath
    if ($LASTEXITCODE -ne 0) {
        Write-Warn "Failed to set executable permission"
    }

    Write-Info "Binary pushed to: $remotePath"
    Write-Host ""
    Write-Host "To run in Termux:" -ForegroundColor Yellow
    Write-Host "  # Copy to Termux home (one-time):" -ForegroundColor Gray
    Write-Host "  cp /data/local/tmp/lexicon ~/" -ForegroundColor White
    Write-Host "  chmod +x ~/lexicon" -ForegroundColor White
    Write-Host ""
    Write-Host "  # Run:" -ForegroundColor Gray
    Write-Host "  ~/lexicon --help" -ForegroundColor White
    Write-Host "  ~/lexicon tui" -ForegroundColor White
}

# Main
if ($Help) {
    Show-Help
}

Write-Host ""
Write-Host "Lexicon TUI - Android Cross-Compilation" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# Validate target
if (-not $TargetMap.ContainsKey($Target)) {
    Write-Error-Custom "Invalid target: $Target. Valid options: $($TargetMap.Keys -join ', ')"
}
$RustTarget = $TargetMap[$Target]

# Find Android NDK
Write-Step "Checking Prerequisites"

$NdkPath = Find-AndroidNdk
if (-not $NdkPath) {
    Write-Error-Custom @"
Android NDK not found!

Please install the Android NDK and set ANDROID_NDK_HOME:

Option 1: Via Android Studio
  1. Open Android Studio > SDK Manager
  2. SDK Tools tab > NDK (Side by side)
  3. Install NDK r26 or later

Option 2: Standalone NDK
  1. Download from: https://developer.android.com/ndk/downloads
  2. Extract to a location (e.g., C:\android-ndk-r26d)
  3. Set environment variable:
     `$env:ANDROID_NDK_HOME = "C:\android-ndk-r26d"

Then run this script again.
"@
}

Write-Info "NDK found: $NdkPath"

# Get API level
$ApiLevel = if ($env:ANDROID_API_LEVEL) { [int]$env:ANDROID_API_LEVEL } else { $DefaultApiLevel }
Write-Info "API Level: $ApiLevel (Android $(8 + ($ApiLevel - 26)).0+)"

# Check Rust target
Write-Info "Checking Rust target..."
Test-RustTarget -Target $RustTarget

# Check LLVM/libclang
if (-not $env:LIBCLANG_PATH) {
    $llvmPaths = @(
        "C:\Program Files\LLVM\bin",
        "C:\Program Files (x86)\LLVM\bin"
    )
    foreach ($path in $llvmPaths) {
        if (Test-Path "$path\libclang.dll") {
            $env:LIBCLANG_PATH = $path
            break
        }
    }
}

if (-not $env:LIBCLANG_PATH) {
    Write-Warn "LIBCLANG_PATH not set - bindgen may fail"
    Write-Warn "Install LLVM: winget install LLVM.LLVM"
} else {
    Write-Info "LIBCLANG_PATH: $env:LIBCLANG_PATH"
}

# Set default features for Android if not specified
if (-not $Features) {
    # Default: storage and STT, no TTS (ONNX is complex for Android)
    # Audio requires PulseAudio in Termux
    $Features = "stt,storage,audio"
    Write-Info "Using default features: $Features"
}

# Build
$binaryPath = Build-AndroidBinary `
    -NdkPath $NdkPath `
    -RustTarget $RustTarget `
    -ApiLevel $ApiLevel `
    -IsRelease $Release `
    -Features $Features

# Push if requested
if ($Push) {
    Push-ToDevice -BinaryPath $binaryPath
}

Write-Host ""
Write-Host "============================================" -ForegroundColor Green
Write-Host "  Build Complete!" -ForegroundColor Green
Write-Host "============================================" -ForegroundColor Green
Write-Host ""
Write-Host "Binary: $binaryPath"
Write-Host ""

if (-not $Push) {
    Write-Host "To push to device:" -ForegroundColor Yellow
    Write-Host "  adb push `"$binaryPath`" /data/local/tmp/lexicon" -ForegroundColor White
    Write-Host ""
}
