# Lexicon Local Server - Build Script (Windows)
#
# This script automatically installs all required dependencies and builds the project.
#
# Usage:
#   .\scripts\build-windows.ps1              # Debug build (installs deps if needed)
#   .\scripts\build-windows.ps1 -Release     # Release build
#   .\scripts\build-windows.ps1 -Clean       # Clean build artifacts
#   .\scripts\build-windows.ps1 -SkipInstall # Skip automatic dependency installation
#
# Requirements (automatically installed if missing):
#   - Rust 1.85+ (via rustup)
#   - CMake (via winget)
#   - LLVM/Clang (via winget)
#   - Visual Studio Build Tools with C++ workload (MSVC linker)

param(
    [switch]$Release,
    [switch]$Clean,
    [switch]$SkipInstall
)

$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ProjectDir = Split-Path -Parent $ScriptDir

Set-Location $ProjectDir

# Minimum required versions
$RequiredRustVersion = "1.85"

# Visual Studio Build Tools configuration
$VsBuildToolsWorkloadArgs = @(
    "--add", "Microsoft.VisualStudio.Workload.VCTools",
    "--add", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
    "--add", "Microsoft.VisualStudio.Component.Windows11SDK.22621"
)

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

function Test-IsAdmin {
    $currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
    return $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Install-Rust {
    Write-Step "Installing Rust"

    $rustupInit = "$env:TEMP\rustup-init.exe"

    Write-Info "Downloading rustup-init.exe..."
    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit -UseBasicParsing
    } catch {
        Write-Error-Custom "Failed to download rustup-init.exe: $_"
    }

    Write-Info "Running rustup-init.exe (this may take a few minutes)..."
    # -y: accept defaults, --default-toolchain stable: install stable toolchain
    & $rustupInit -y --default-toolchain stable
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Rust installation failed"
    }

    # Add cargo to PATH for current session
    $cargoPath = "$env:USERPROFILE\.cargo\bin"
    if ($env:PATH -notlike "*$cargoPath*") {
        $env:PATH = "$cargoPath;$env:PATH"
    }

    # Clean up
    Remove-Item $rustupInit -ErrorAction SilentlyContinue

    Write-Info "Rust installed successfully"
}

function Update-Rust {
    Write-Info "Updating Rust toolchain..."
    rustup update stable
    if ($LASTEXITCODE -ne 0) {
        Write-Warn "Rust update failed, continuing with existing version"
    }
}

function Install-WithWinget {
    param(
        [string]$PackageId,
        [string]$PackageName
    )

    Write-Info "Installing $PackageName via winget..."

    # Check if winget is available
    if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
        Write-Warn "winget not available. Please install $PackageName manually."
        Write-Warn "  Package ID: $PackageId"
        return $false
    }

    # Install package
    winget install --id $PackageId --accept-source-agreements --accept-package-agreements --silent
    if ($LASTEXITCODE -ne 0) {
        Write-Warn "winget install may have failed or package already installed"
    }

    return $true
}

function Install-CMake {
    Write-Step "Installing CMake"

    if (Install-WithWinget "Kitware.CMake" "CMake") {
        # Add CMake to PATH for current session
        $cmakePaths = @(
            "C:\Program Files\CMake\bin",
            "C:\Program Files (x86)\CMake\bin"
        )
        foreach ($path in $cmakePaths) {
            if (Test-Path $path) {
                if ($env:PATH -notlike "*$path*") {
                    $env:PATH = "$path;$env:PATH"
                }
                Write-Info "CMake installed at: $path"
                return
            }
        }
    }
}

function Install-LLVM {
    Write-Step "Installing LLVM/Clang"

    if (Install-WithWinget "LLVM.LLVM" "LLVM") {
        # Set LIBCLANG_PATH for current session
        $llvmPaths = @(
            "C:\Program Files\LLVM\bin",
            "C:\Program Files (x86)\LLVM\bin"
        )
        foreach ($path in $llvmPaths) {
            if (Test-Path $path) {
                $env:LIBCLANG_PATH = $path
                if ($env:PATH -notlike "*$path*") {
                    $env:PATH = "$path;$env:PATH"
                }
                Write-Info "LLVM installed at: $path"
                Write-Info "LIBCLANG_PATH set to: $path"
                return
            }
        }
    }
}

function Find-VsInstallation {
    # Try to find Visual Studio installation using vswhere
    $vswherePaths = @(
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe",
        "${env:ProgramFiles}\Microsoft Visual Studio\Installer\vswhere.exe"
    )

    foreach ($vswhere in $vswherePaths) {
        if (Test-Path $vswhere) {
            $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
            if ($vsPath -and (Test-Path $vsPath)) {
                return $vsPath
            }
        }
    }

    # Fallback: check common installation paths
    $commonPaths = @(
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\BuildTools",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Community",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Professional",
        "${env:ProgramFiles}\Microsoft Visual Studio\2022\Enterprise",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2019\BuildTools",
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\2019\Community"
    )

    foreach ($path in $commonPaths) {
        $vcTools = Join-Path $path "VC\Tools\MSVC"
        if (Test-Path $vcTools) {
            return $path
        }
    }

    return $null
}

function Test-MsvcToolchain {
    $vsPath = Find-VsInstallation
    if (-not $vsPath) {
        return $false
    }

    # Check if VC tools exist
    $vcToolsPath = Join-Path $vsPath "VC\Tools\MSVC"
    if (-not (Test-Path $vcToolsPath)) {
        return $false
    }

    # Check if there's at least one toolset version
    $toolsets = Get-ChildItem -Path $vcToolsPath -Directory -ErrorAction SilentlyContinue
    return ($toolsets.Count -gt 0)
}

function Install-VsBuildTools {
    Write-Step "Installing Visual Studio Build Tools"

    # Check if already installed
    if (Test-MsvcToolchain) {
        Write-Info "Visual Studio Build Tools already installed"
        return $true
    }

    Write-Info "Downloading Visual Studio Build Tools installer..."

    $installerUrl = "https://aka.ms/vs/17/release/vs_buildtools.exe"
    $installerPath = "$env:TEMP\vs_buildtools.exe"

    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $installerUrl -OutFile $installerPath -UseBasicParsing
    } catch {
        Write-Warn "Failed to download VS Build Tools installer: $_"
        Write-Warn "Please install manually from: https://visualstudio.microsoft.com/visual-cpp-build-tools/"
        return $false
    }

    Write-Info "Installing Visual Studio Build Tools (this may take several minutes)..."
    Write-Info "Installing components: VC Tools, x64/x86 build tools, Windows SDK"

    # Build the argument list
    $installArgs = @(
        "--quiet",
        "--wait",
        "--norestart"
    ) + $VsBuildToolsWorkloadArgs

    try {
        $process = Start-Process -FilePath $installerPath -ArgumentList $installArgs -Wait -PassThru

        # Exit codes: 0 = success, 3010 = success but reboot required
        if ($process.ExitCode -eq 0 -or $process.ExitCode -eq 3010) {
            Write-Info "Visual Studio Build Tools installed successfully"
            if ($process.ExitCode -eq 3010) {
                Write-Warn "A system restart may be required for full functionality"
            }
            return $true
        } else {
            Write-Warn "VS Build Tools installer exited with code: $($process.ExitCode)"
            return $false
        }
    } catch {
        Write-Warn "Failed to run VS Build Tools installer: $_"
        return $false
    } finally {
        Remove-Item $installerPath -ErrorAction SilentlyContinue
    }
}

function Initialize-VsEnvironment {
    # Find VS installation
    $vsPath = Find-VsInstallation
    if (-not $vsPath) {
        return $false
    }

    # Find vcvarsall.bat
    $vcvarsall = Join-Path $vsPath "VC\Auxiliary\Build\vcvarsall.bat"
    if (-not (Test-Path $vcvarsall)) {
        Write-Warn "vcvarsall.bat not found at: $vcvarsall"
        return $false
    }

    Write-Info "Initializing Visual Studio environment..."

    # Run vcvarsall.bat and capture environment variables
    $arch = "x64"
    $cmd = "`"$vcvarsall`" $arch >nul 2>&1 && set"

    try {
        $envVars = cmd /c $cmd
        foreach ($line in $envVars) {
            if ($line -match "^([^=]+)=(.*)$") {
                $varName = $Matches[1]
                $varValue = $Matches[2]
                [Environment]::SetEnvironmentVariable($varName, $varValue, "Process")
            }
        }
        Write-Info "Visual Studio environment initialized for $arch"
        return $true
    } catch {
        Write-Warn "Failed to initialize VS environment: $_"
        return $false
    }
}

function Test-AndInstallDependencies {
    Write-Step "Checking and Installing Dependencies"

    $needsPathRefresh = $false

    # Check and install Rust
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        if ($SkipInstall) {
            Write-Error-Custom "Rust/Cargo not found. Run without -SkipInstall to install automatically, or install from https://rustup.rs"
        }
        Install-Rust
        $needsPathRefresh = $true
    } else {
        # Check Rust version
        $rustVersionOutput = rustc --version
        if ($rustVersionOutput -match "rustc (\d+\.\d+)") {
            $rustVersion = $Matches[1]
            if ([version]$rustVersion -lt [version]$RequiredRustVersion) {
                Write-Warn "Rust $RequiredRustVersion+ required, found $rustVersion"
                if (-not $SkipInstall) {
                    Update-Rust
                }
            } else {
                Write-Info "Rust $rustVersion OK"
            }
        }
    }

    # Check and install CMake
    if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
        if ($SkipInstall) {
            Write-Warn "CMake not found - whisper-rs-sys may fail to build"
            Write-Warn "Install with: winget install Kitware.CMake"
        } else {
            Install-CMake
            $needsPathRefresh = $true
        }
    } else {
        $cmakeVersion = (cmake --version | Select-Object -First 1) -replace "cmake version ", ""
        Write-Info "CMake $cmakeVersion OK"
    }

    # Check and set LIBCLANG_PATH / install LLVM
    if (-not $env:LIBCLANG_PATH) {
        $llvmPaths = @(
            "C:\Program Files\LLVM\bin",
            "C:\Program Files (x86)\LLVM\bin"
        )
        $found = $false
        foreach ($path in $llvmPaths) {
            if (Test-Path "$path\libclang.dll") {
                $env:LIBCLANG_PATH = $path
                Write-Info "Found LLVM at: $path"
                $found = $true
                break
            }
        }

        if (-not $found) {
            if ($SkipInstall) {
                Write-Warn "LLVM/Clang not found - bindgen may fail"
                Write-Warn "Install with: winget install LLVM.LLVM"
            } else {
                Install-LLVM
                $needsPathRefresh = $true
            }
        }
    } else {
        Write-Info "LIBCLANG_PATH=$env:LIBCLANG_PATH OK"
    }

    # Check and install Visual Studio Build Tools (MSVC linker)
    if (-not (Test-MsvcToolchain)) {
        if ($SkipInstall) {
            Write-Error-Custom "Visual Studio Build Tools not found. The MSVC linker (link.exe) is required."
            Write-Warn "Install Visual Studio Build Tools from: https://visualstudio.microsoft.com/visual-cpp-build-tools/"
            Write-Warn "Or run this script without -SkipInstall to install automatically"
        } else {
            $installed = Install-VsBuildTools
            if (-not $installed) {
                Write-Error-Custom "Failed to install Visual Studio Build Tools. Please install manually."
            }
            $needsPathRefresh = $true
        }
    } else {
        $vsPath = Find-VsInstallation
        Write-Info "Visual Studio Build Tools OK ($vsPath)"
    }

    # Verify dependencies after installation
    if ($needsPathRefresh) {
        Write-Info ""
        Write-Info "Dependencies installed. Verifying..."

        # Refresh PATH from registry for current session
        $machinePath = [Environment]::GetEnvironmentVariable("Path", "Machine")
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        $env:PATH = "$machinePath;$userPath"

        # Add common paths
        $commonPaths = @(
            "$env:USERPROFILE\.cargo\bin",
            "C:\Program Files\CMake\bin",
            "C:\Program Files\LLVM\bin"
        )
        foreach ($path in $commonPaths) {
            if ((Test-Path $path) -and ($env:PATH -notlike "*$path*")) {
                $env:PATH = "$path;$env:PATH"
            }
        }
    }

    # Final verification
    $allOk = $true

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Warn "Rust still not found in PATH. You may need to restart your terminal."
        $allOk = $false
    }

    if (-not (Get-Command cmake -ErrorAction SilentlyContinue)) {
        Write-Warn "CMake still not found in PATH. Build may fail."
        $allOk = $false
    }

    if (-not $env:LIBCLANG_PATH -or -not (Test-Path "$env:LIBCLANG_PATH\libclang.dll")) {
        Write-Warn "LLVM/libclang not properly configured. Build may fail."
        $allOk = $false
    }

    if (-not (Test-MsvcToolchain)) {
        Write-Warn "Visual Studio Build Tools not found. Build will fail."
        Write-Warn "You may need to restart your terminal after installation."
        $allOk = $false
    }

    if ($allOk) {
        Write-Info "All dependencies OK"
    } else {
        Write-Warn "Some dependencies may need manual configuration"
        Write-Warn "Try restarting your terminal and running the script again"
    }
}

function Build-Debug {
    Write-Step "Building Debug Configuration"

    # Initialize VS environment if needed
    if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
        if (-not (Initialize-VsEnvironment)) {
            Write-Error-Custom "Failed to initialize Visual Studio environment. Cannot find MSVC linker."
        }
    }

    cargo build --workspace --all-features
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Build failed"
    }
    Write-Info "Debug build complete: target\debug\"
}

function Build-Release {
    Write-Step "Building Release Configuration"

    # Initialize VS environment if needed
    if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
        if (-not (Initialize-VsEnvironment)) {
            Write-Error-Custom "Failed to initialize Visual Studio environment. Cannot find MSVC linker."
        }
    }

    cargo build --workspace --all-features --release
    if ($LASTEXITCODE -ne 0) {
        Write-Error-Custom "Build failed"
    }
    Write-Info "Release build complete: target\release\"
}

function Clean-Build {
    Write-Step "Cleaning Build Artifacts"
    cargo clean
    Write-Info "Clean complete"
}

function Show-Summary {
    Write-Host ""
    Write-Host "============================================" -ForegroundColor Green
    Write-Host "  Build Complete!" -ForegroundColor Green
    Write-Host "============================================" -ForegroundColor Green
    Write-Host ""

    if ($Release) {
        Write-Host "  Output: target\release\lexicon.exe"
    } else {
        Write-Host "  Output: target\debug\lexicon.exe"
    }
    Write-Host ""
}

# Main
Write-Host ""
Write-Host "Lexicon Local Server - Windows Build Script" -ForegroundColor Cyan
Write-Host "============================================" -ForegroundColor Cyan

if ($Clean) {
    Clean-Build
} elseif ($Release) {
    Test-AndInstallDependencies
    Build-Release
    Show-Summary
} else {
    Test-AndInstallDependencies
    Build-Debug
    Show-Summary
}

Write-Info "Done!"
