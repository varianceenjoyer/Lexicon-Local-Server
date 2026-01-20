# Lexicon Local Server - Test Script (Windows)
#
# Usage:
#   .\scripts\test-windows.ps1          # Run all tests
#   .\scripts\test-windows.ps1 -Quick   # Run quick tests only
#   .\scripts\test-windows.ps1 -Package core  # Run specific package tests

param(
    [switch]$Quick,
    [string]$Package = ""
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

function Test-All {
    Write-Info "Running all tests..."
    cargo test --workspace --all-features
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Tests failed"
    }
    Write-Info "All tests passed!"
}

function Test-Quick {
    Write-Info "Running quick tests (lexicon-core only)..."
    cargo test -p lexicon-core --all-features
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Tests failed"
    }
    Write-Info "Quick tests passed!"
}

function Test-Package {
    param([string]$PackageName)
    Write-Info "Running $PackageName tests..."
    cargo test -p "lexicon-$PackageName" --all-features
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Tests failed"
    }
    Write-Info "$PackageName tests passed!"
}

# Main
if ($Quick) {
    Test-Quick
} elseif ($Package -ne "") {
    Test-Package -PackageName $Package
} else {
    Test-All
}

Write-Info "Done!"
