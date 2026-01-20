# Lexicon Local Server - Build Script (Windows)
#
# Usage:
#   .\scripts\build-windows.ps1          # Debug build
#   .\scripts\build-windows.ps1 -Release # Release build
#   .\scripts\build-windows.ps1 -Clean   # Clean build artifacts

param(
    [switch]$Release,
    [switch]$Clean
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir

Set-Location $ProjectDir

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

function Test-Dependencies {
    Write-Info "Checking dependencies..."

    # Check Rust
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Error-Custom "Rust/Cargo not found. Install from https://rustup.rs"
    }

    # Check Rust version
    $rustVersion = (rustc --version) -replace "rustc (\d+\.\d+).*", '$1'
    $requiredVersion = "1.85"
    if ([version]$rustVersion -lt [version]$requiredVersion) {
        Write-Error-Custom "Rust $requiredVersion+ required, found $rustVersion"
    }

    # Check cmake
    if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
        Write-Warn "cmake not found - whisper-rs-sys may fail to build"
        Write-Warn "Install with: winget install Kitware.CMake"
    }

    # Check LIBCLANG_PATH
    if (-not $env:LIBCLANG_PATH) {
        $defaultLlvmPath = "C:\Program Files\LLVM\bin"
        if (Test-Path $defaultLlvmPath) {
            $env:LIBCLANG_PATH = $defaultLlvmPath
            Write-Info "Set LIBCLANG_PATH=$defaultLlvmPath"
        } else {
            Write-Warn "LIBCLANG_PATH not set - bindgen may fail"
            Write-Warn "Install LLVM: winget install LLVM.LLVM"
        }
    }

    Write-Info "Dependencies OK"
}

function Build-Debug {
    Write-Info "Building debug configuration..."
    cargo build --workspace --all-features
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Build failed"
    }
    Write-Info "Debug build complete: target\debug\"
}

function Build-Release {
    Write-Info "Building release configuration..."
    cargo build --workspace --all-features --release
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Build failed"
    }
    Write-Info "Release build complete: target\release\"
}

function Clean-Build {
    Write-Info "Cleaning build artifacts..."
    cargo clean
    Write-Info "Clean complete"
}

# Main
if ($Clean) {
    Clean-Build
} elseif ($Release) {
    Test-Dependencies
    Build-Release
} else {
    Test-Dependencies
    Build-Debug
}

Write-Info "Done!"
