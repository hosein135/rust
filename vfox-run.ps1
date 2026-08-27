#Requires -RunAsAdministrator
# =============================================================================
# vfox-run.ps1 - Windows setup + run for Verilog IDE (winget -> vfox -> rust)
#
# WinGet prerequisites (installed only when missing - no re-download):
#   1. Administrator elevation (this script: #Requires -RunAsAdministrator)
#   2. Windows 10 version 1809+ / Windows 11 (or Server/LTSC with AppX enabled)
#   3. Internet access (GitHub + aka.ms) for first-time App Installer fetch
#   4. AppX / MSIX deployment allowed (not blocked by policy)
#   5. Microsoft.VCLibs.*.14.00.Desktop  (VC++ UWP Desktop framework)
#   6. Microsoft.UI.Xaml.2.8             (WinUI 2.8 appx)
#   7. Microsoft.DesktopAppInstaller    (contains winget.exe)
#      If winget is already on PATH (any version), skip installing winget and
#      all of the above AppX prerequisites. Pin v1.29.280 is used only when
#      winget is missing.
#
# Then:
#   8. vfox (via winget) + tools from .vfox.toml (rust, with retry + stable fallbacks)
#   9. Visual Studio 2022 Build Tools (MSVC linker for pc-windows-msvc)
#  10. cargo build / cargo run  (Ctrl+C stops the IDE process tree)
#
# Usage (elevated PowerShell):
#   .\vfox-run.ps1
#   .\vfox-run.ps1 -ForceSetup
#   .\vfox-run.ps1 -PrepOnly
#   .\vfox-run.ps1 -Build
#   .\vfox-run.ps1 -Release
# =============================================================================
[CmdletBinding()]
param(
    [switch]$ForceSetup,
    [switch]$PrepOnly,
    [switch]$Build,
    [switch]$Release,
    # Windows Package Manager (winget-cli) release tag - only used when winget is missing
    # https://github.com/microsoft/winget-cli/releases
    [string]$WingetVersion = "1.29.280",
    # https://github.com/version-fox/vfox/releases - winget id version-fox.vfox
    [string]$VfoxVersion = "1.0.11",
    [string]$VfoxPackageId = "version-fox.vfox",
    [string]$VsBuildToolsId = "Microsoft.VisualStudio.2022.BuildTools"
)

$ErrorActionPreference = "Stop"

Set-Location -LiteralPath $PSScriptRoot
Write-Host "Working directory: $PSScriptRoot" -ForegroundColor Cyan

# ---------------------------------------------------------------------------
# Logging
# ---------------------------------------------------------------------------
function Write-Info  { param([string]$Message) Write-Host "[verilog-ide]  $Message" -ForegroundColor Green }
function Write-Warn2 { param([string]$Message) Write-Host "[verilog-ide]  $Message" -ForegroundColor Yellow }
function Write-Err2  { param([string]$Message) Write-Host "[verilog-ide]  $Message" -ForegroundColor Red }
function Write-Step  { param([string]$Message) Write-Host "[verilog-ide]  $Message" -ForegroundColor Cyan }

function Format-ByteSize {
    param([long]$Bytes)
    if ($Bytes -ge 1GB) { return ("{0:N2} GB" -f ($Bytes / 1GB)) }
    if ($Bytes -ge 1MB) { return ("{0:N2} MB" -f ($Bytes / 1MB)) }
    if ($Bytes -ge 1KB) { return ("{0:N1} KB" -f ($Bytes / 1KB)) }
    return "$Bytes B"
}

function Save-UrlToFile {
    <#
    .SYNOPSIS
      Download a URL to disk with a Write-Progress bar (bytes + %).
    #>
    param(
        [Parameter(Mandatory = $true)][string]$Uri,
        [Parameter(Mandatory = $true)][string]$OutFile,
        [string]$Label = "download"
    )

    $parent = Split-Path -Parent $OutFile
    if ($parent) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }

    Write-Step "Downloading $Label ..."
    Write-Host "  $Uri" -ForegroundColor DarkGray

    $request = [System.Net.HttpWebRequest]::Create($Uri)
    $request.UserAgent = "verilog-ide-vfox-run"
    $request.AllowAutoRedirect = $true
    $request.Timeout = 1000 * 60 * 10
    $request.ReadWriteTimeout = 1000 * 60 * 10

    $response = $null
    $inStream = $null
    $outStream = $null
    try {
        $response = $request.GetResponse()
        $total = [long]$response.ContentLength
        $inStream = $response.GetResponseStream()
        $outStream = [System.IO.File]::Create($OutFile)

        $buffer = New-Object byte[] (256KB)
        $received = [long]0
        $lastPct = -1
        $sw = [System.Diagnostics.Stopwatch]::StartNew()

        while ($true) {
            $read = $inStream.Read($buffer, 0, $buffer.Length)
            if ($read -le 0) { break }
            $outStream.Write($buffer, 0, $read)
            $received += $read

            $elapsedSec = [Math]::Max($sw.Elapsed.TotalSeconds, 0.001)
            $speed = $received / $elapsedSec

            if ($total -gt 0) {
                $pct = [int][Math]::Min(100, ($received * 100) / $total)
                if ($pct -ne $lastPct -or ($received % (512KB)) -lt $read) {
                    $lastPct = $pct
                    $status = "{0} / {1}  ({2}/s)" -f (Format-ByteSize $received), (Format-ByteSize $total), (Format-ByteSize ([long]$speed))
                    Write-Progress -Activity "Downloading $Label" -Status $status -PercentComplete $pct
                    $barWidth = 28
                    $filled = [int](($barWidth * $pct) / 100)
                    $bar = ("#" * $filled).PadRight($barWidth, "-")
                    Write-Host ("`r  [{0}] {1,3}%  {2}" -f $bar, $pct, $status) -NoNewline
                }
            } else {
                $status = "{0} received  ({1}/s)" -f (Format-ByteSize $received), (Format-ByteSize ([long]$speed))
                Write-Progress -Activity "Downloading $Label" -Status $status -PercentComplete -1
                Write-Host ("`r  [---------- unknown size ----------]  {0}" -f $status) -NoNewline
            }
        }

        Write-Host ""
        Write-Progress -Activity "Downloading $Label" -Completed
        Write-Info ("Downloaded {0} ({1})" -f $Label, (Format-ByteSize $received))
    } catch {
        Write-Host ""
        Write-Progress -Activity "Downloading $Label" -Completed
        if (Test-Path -LiteralPath $OutFile) {
            Remove-Item -LiteralPath $OutFile -Force -ErrorAction SilentlyContinue
        }
        throw "Download failed for ${Label}: $($_.Exception.Message)"
    } finally {
        if ($outStream) { $outStream.Dispose() }
        if ($inStream) { $inStream.Dispose() }
        if ($response) { $response.Dispose() }
    }
}

# ---------------------------------------------------------------------------
# Runtime state (Ctrl+C cleanup)
# ---------------------------------------------------------------------------
$script:IdeData = Join-Path $PSScriptRoot ".verilog-ide-data"
$script:AppProc = $null
$script:CleaningUp = $false

function Refresh-Path {
    $machine = [System.Environment]::GetEnvironmentVariable("Path", "Machine")
    $user = [System.Environment]::GetEnvironmentVariable("Path", "User")
    $env:Path = @($machine, $user, $env:Path) -join ";"
}

function Add-PathFront {
    param([string]$Dir)
    if (-not $Dir) { return }
    if (-not (Test-Path -LiteralPath $Dir)) { return }
    $parts = $env:Path -split ";" | Where-Object { $_ -and ($_ -ne $Dir) }
    $env:Path = (@($Dir) + $parts) -join ";"
}

function Test-Command {
    param([string]$Name)
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-AppxPackageInstalled {
    param(
        [Parameter(Mandatory = $true)][string]$NamePattern,
        [string]$MinVersion = $null
    )
    $pkgs = @(Get-AppxPackage -Name $NamePattern -ErrorAction SilentlyContinue)
    if ($pkgs.Count -eq 0) { return $false }
    if (-not $MinVersion) { return $true }
    foreach ($p in $pkgs) {
        try {
            if ([version]$p.Version -ge [version]$MinVersion) { return $true }
        } catch {
            return $true
        }
    }
    return $false
}

function Show-WingetPrerequisites {
    Write-Step "WinGet prerequisites (only if winget is missing; then pin v$WingetVersion):"
    Write-Host "  1. Administrator elevation" -ForegroundColor DarkGray
    Write-Host "  2. Windows 10 1809+ / Windows 11 (AppX/MSIX enabled)" -ForegroundColor DarkGray
    Write-Host "  3. Internet (first-time only: GitHub + aka.ms)" -ForegroundColor DarkGray
    Write-Host "  4. Microsoft.VCLibs.*.14.00.Desktop" -ForegroundColor DarkGray
    Write-Host "  5. Microsoft.UI.Xaml.2.8" -ForegroundColor DarkGray
    Write-Host "  6. Microsoft.DesktopAppInstaller (winget) v$WingetVersion" -ForegroundColor DarkGray
    Write-Host "  Note: if winget is already installed (any version), skip all of the above." -ForegroundColor DarkGray
}

function Ensure-Winget {
    Show-WingetPrerequisites
    Refresh-Path

    if (Test-Command winget) {
        $have = Get-InstalledWingetVersion
        if ($have) {
            Write-Info "WinGet already installed (v$have) - skip winget + all prerequisites (no download)"
        } else {
            Write-Info "WinGet already on PATH - skip winget + all prerequisites (no download)"
        }
        return
    }

    Write-Warn2 "WinGet not found - installing prerequisites + WinGet v$WingetVersion ..."
    Install-WingetExactVersion -Version $WingetVersion
    Refresh-Path
}

# ---------------------------------------------------------------------------
# 1-2. WinGet dependencies + pinned WinGet version
# ---------------------------------------------------------------------------
function Get-CpuArchLabel {
    switch ($env:PROCESSOR_ARCHITECTURE) {
        "ARM64" { return "arm64" }
        "x86"   { return "x86" }
        default { return "x64" }
    }
}

function Test-VclibsInstalled {
    return (
        (Test-AppxPackageInstalled -NamePattern "Microsoft.VCLibs*Desktop*") -or
        (Test-AppxPackageInstalled -NamePattern "Microsoft.VCLibs.140.00.UWPDesktop")
    )
}

function Test-UiXamlInstalled {
    return (Test-AppxPackageInstalled -NamePattern "Microsoft.UI.Xaml.2.8")
}

function Install-WingetDependencyPackages {
    $arch = Get-CpuArchLabel
    $tmp = Join-Path $env:TEMP "verilog-ide-winget-deps"
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null

    Write-Step "Checking WinGet dependency packages ..."

    if ((-not $ForceSetup) -and (Test-VclibsInstalled)) {
        $pkg = Get-AppxPackage -Name "Microsoft.VCLibs*Desktop*" -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if (-not $pkg) {
            $pkg = Get-AppxPackage -Name "Microsoft.VCLibs.140.00.UWPDesktop" -ErrorAction SilentlyContinue |
                Select-Object -First 1
        }
        Write-Info ("VCLibs already installed - skip download ({0} {1})" -f $pkg.Name, $pkg.Version)
    } else {
        $vclibsUrl = "https://aka.ms/Microsoft.VCLibs.$arch.14.00.Desktop.appx"
        $vclibsPath = Join-Path $tmp "Microsoft.VCLibs.$arch.14.00.Desktop.appx"
        try {
            Save-UrlToFile -Uri $vclibsUrl -OutFile $vclibsPath -Label "VCLibs ($arch)"
            Add-AppxPackage -Path $vclibsPath -ErrorAction SilentlyContinue
            Write-Info "VCLibs installed"
        } catch {
            Write-Warn2 "VCLibs install skipped/failed: $($_.Exception.Message)"
        }
    }

    if ((-not $ForceSetup) -and (Test-UiXamlInstalled)) {
        $pkg = Get-AppxPackage -Name "Microsoft.UI.Xaml.2.8" -ErrorAction SilentlyContinue |
            Select-Object -First 1
        Write-Info ("Microsoft.UI.Xaml.2.8 already installed - skip download (v{0})" -f $pkg.Version)
    } else {
        $xamlVer = "2.8.6"
        $xamlUrl = "https://github.com/microsoft/microsoft-ui-xaml/releases/download/v$xamlVer/Microsoft.UI.Xaml.2.8.$arch.appx"
        $xamlPath = Join-Path $tmp "Microsoft.UI.Xaml.2.8.$arch.appx"
        try {
            Save-UrlToFile -Uri $xamlUrl -OutFile $xamlPath -Label "Microsoft.UI.Xaml $xamlVer ($arch)"
            Add-AppxPackage -Path $xamlPath -ErrorAction SilentlyContinue
            Write-Info "Microsoft.UI.Xaml $xamlVer installed"
        } catch {
            Write-Warn2 "UI.Xaml install skipped/failed: $($_.Exception.Message)"
        }
    }
}

function Get-InstalledWingetVersion {
    if (-not (Test-Command winget)) { return $null }
    try {
        $out = & winget --version 2>$null
        if ($out -match "v?(\d+\.\d+\.\d+)") { return $Matches[1] }
        return ($out | Out-String).Trim().TrimStart("v")
    } catch {
        return $null
    }
}

function Install-WingetExactVersion {
    param([string]$Version)

    $want = $Version.TrimStart("v")
    $have = Get-InstalledWingetVersion
    if ((-not $ForceSetup) -and $have -and ($have -eq $want) -and (Test-Command winget)) {
        Write-Info "WinGet already at v$have - skip download"
        return
    }

    Install-WingetDependencyPackages

    if ((-not $ForceSetup) -and (Test-Command winget) -and $have -eq $want) {
        Write-Info "WinGet already at v$have after dependency check - skip App Installer download"
        return
    }

    $tag = "v$want"
    Write-Step "Installing WinGet (App Installer) $tag ..."
    $api = "https://api.github.com/repos/microsoft/winget-cli/releases/tags/$tag"
    try {
        $release = Invoke-RestMethod -Uri $api -Headers @{ "User-Agent" = "verilog-ide-vfox-run" }
    } catch {
        throw "Could not fetch winget-cli release $tag from GitHub. $($_.Exception.Message)"
    }

    $bundle = $release.assets | Where-Object { $_.name -like "*.msixbundle" } | Select-Object -First 1
    if (-not $bundle) {
        throw "No .msixbundle asset found on winget-cli $tag"
    }

    $tmp = Join-Path $env:TEMP "verilog-ide-winget-install"
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    $bundlePath = Join-Path $tmp $bundle.name
    Save-UrlToFile -Uri $bundle.browser_download_url -OutFile $bundlePath -Label $bundle.name

    $depsZip = $release.assets | Where-Object { $_.name -like "*Dependencies*.zip" } | Select-Object -First 1
    if ($depsZip) {
        $depsPath = Join-Path $tmp $depsZip.name
        Save-UrlToFile -Uri $depsZip.browser_download_url -OutFile $depsPath -Label $depsZip.name
        $depsDir = Join-Path $tmp "deps"
        if (Test-Path $depsDir) { Remove-Item $depsDir -Recurse -Force }
        Expand-Archive -Path $depsPath -DestinationPath $depsDir -Force
        $arch = Get-CpuArchLabel
        Get-ChildItem -Path $depsDir -Recurse -Include "*.appx", "*.msix" -ErrorAction SilentlyContinue |
            Where-Object { $_.FullName -match [regex]::Escape($arch) -or $_.Name -match "VCLibs|UI\.Xaml" } |
            ForEach-Object {
                try { Add-AppxPackage -Path $_.FullName -ErrorAction SilentlyContinue } catch { }
            }
    }

    $license = $release.assets | Where-Object { $_.name -like "*License*.xml" } | Select-Object -First 1
    if ($license) {
        $licensePath = Join-Path $tmp $license.name
        Save-UrlToFile -Uri $license.browser_download_url -OutFile $licensePath -Label $license.name
        try {
            Add-AppxProvisionedPackage -Online -PackagePath $bundlePath -LicensePath $licensePath -ErrorAction Stop | Out-Null
        } catch {
            Add-AppxPackage -Path $bundlePath
        }
    } else {
        Add-AppxPackage -Path $bundlePath
    }

    Refresh-Path
    Start-Sleep -Seconds 2
    if (-not (Test-Command winget)) {
        throw "WinGet installed but winget.exe is not on PATH. Open a new elevated PowerShell and re-run."
    }
    Write-Info "WinGet installed: $(winget --version)"
}

# ---------------------------------------------------------------------------
# 3. vfox via winget
# ---------------------------------------------------------------------------
function Resolve-VfoxPath {
    Refresh-Path
    if (Test-Command vfox) { return (Get-Command vfox).Source }

    $wingetPackagesRoot = Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Packages"
    if (Test-Path $wingetPackagesRoot) {
        $exe = Get-ChildItem -Path $wingetPackagesRoot -Recurse -Filter "vfox.exe" -ErrorAction SilentlyContinue |
            Select-Object -First 1
        if ($exe) {
            Add-PathFront $exe.DirectoryName
            return $exe.FullName
        }
    }

    foreach ($p in @(
            (Join-Path $env:LOCALAPPDATA "vfox"),
            (Join-Path $HOME "AppData\Local\vfox"),
            (Join-Path $env:ProgramFiles "vfox")
        )) {
        $candidate = Join-Path $p "vfox.exe"
        if (Test-Path $candidate) {
            Add-PathFront $p
            return $candidate
        }
    }
    return $null
}

function Test-WingetPackageInstalled {
    param(
        [Parameter(Mandatory = $true)][string]$Id,
        [string]$Version = $null
    )
    if (-not (Test-Command winget)) { return $false }
    $prev = $ProgressPreference
    $ProgressPreference = "SilentlyContinue"
    try {
        $out = & winget list --id $Id --exact --accept-source-agreements 2>$null | Out-String
        if ($LASTEXITCODE -ne 0 -and -not ($out -match [regex]::Escape($Id))) {
            return $false
        }
        if (-not ($out -match [regex]::Escape($Id))) { return $false }
        if ($Version -and -not ($out -match [regex]::Escape($Version))) { return $false }
        return $true
    } catch {
        return $false
    } finally {
        $ProgressPreference = $prev
    }
}

function Ensure-Vfox {
    $resolved = Resolve-VfoxPath
    $wingetHas = Test-WingetPackageInstalled -Id $VfoxPackageId -Version $VfoxVersion

    if ($resolved -and $wingetHas -and -not $ForceSetup) {
        Write-Info "vfox already installed - skip download: $resolved"
    } elseif ($resolved -and -not $ForceSetup) {
        Write-Info "vfox binary already on PATH - skip download: $resolved"
    } else {
        Write-Step "Installing vfox $VfoxVersion via winget ($VfoxPackageId) ..."
        & winget install --id $VfoxPackageId --version $VfoxVersion --exact `
            --accept-source-agreements --accept-package-agreements
        Refresh-Path
        $resolved = Resolve-VfoxPath
    }

    if (-not $resolved) {
        throw "vfox executable could not be resolved after winget install."
    }

    Write-Info "Activating vfox PowerShell hook ..."
    Invoke-Expression "$( & vfox activate pwsh )"
}

# ---------------------------------------------------------------------------
# 4. .vfox.toml tools (rust)
# ---------------------------------------------------------------------------
function Get-VfoxTools {
    $toml = Join-Path $PSScriptRoot ".vfox.toml"
    if (-not (Test-Path $toml)) {
        throw "Missing .vfox.toml in $PSScriptRoot"
    }
    $inTools = $false
    $map = [ordered]@{}
    Get-Content -LiteralPath $toml | ForEach-Object {
        $line = ($_ -replace "#.*$", "").Trim()
        if (-not $line) { return }
        if ($line -eq "[tools]") { $inTools = $true; return }
        if ($line.StartsWith("[")) { $inTools = $false; return }
        if ($inTools -and $line -match '^\s*([A-Za-z0-9_-]+)\s*=\s*"([^"]+)"\s*$') {
            $map[$Matches[1]] = $Matches[2]
        }
    }
    if ($map.Count -eq 0) { throw ".vfox.toml has no [tools] entries" }
    return $map
}

function Test-VfoxPlugin {
    param([string]$Name)
    try {
        & vfox info $Name 2>$null | Out-Null
        return ($LASTEXITCODE -eq 0)
    } catch {
        return $false
    }
}

function Test-VfoxSdkInstalled {
    param([string]$Name, [string]$Version)
    try {
        $list = & vfox list $Name 2>$null | Out-String
        # Match "1.88.0" as a whole version token (vfox marks current with ->)
        if ($list -match "(?m)(?:^|\s|->\s*)$([regex]::Escape($Version))(?:\s|<|$)") {
            return $true
        }
        if ($list -match [regex]::Escape($Version)) { return $true }
    } catch { }
    $sdk = Join-Path $PSScriptRoot ".vfox\sdks\$Name"
    if (Test-Path $sdk) {
        try {
            $item = Get-Item $sdk -ErrorAction SilentlyContinue
            $target = $null
            if ($item.LinkType) { $target = $item.Target }
            if (-not $target) { $target = $item.FullName }
            if ($target -and ("$target" -match [regex]::Escape($Version))) { return $true }
        } catch { }
    }
    # Usable rustc already at this version
    if ($Name -eq "rust" -and (Test-Command rustc)) {
        $rv = (& rustc --version 2>$null | Out-String)
        if ($rv -match [regex]::Escape($Version)) { return $true }
    }
    return $false
}

function Get-VfoxInstalledVersions {
    param([string]$Name)
    $found = [System.Collections.Generic.List[string]]::new()
    try {
        $list = & vfox list $Name 2>$null | Out-String
        [regex]::Matches($list, "\d+\.\d+\.\d+") | ForEach-Object {
            if (-not $found.Contains($_.Value)) { [void]$found.Add($_.Value) }
        }
    } catch { }
    return @($found)
}

function Clear-VfoxRustCache {
    param([string]$Version)
    $roots = @(
        (Join-Path $env:USERPROFILE ".vfox\cache\rust"),
        (Join-Path $env:USERPROFILE ".version-fox\cache\rust")
    )
    foreach ($root in $roots) {
        if (-not (Test-Path $root)) { continue }
        Get-ChildItem $root -ErrorAction SilentlyContinue | Where-Object {
            $_.Name -match [regex]::Escape($Version) -or $_.Name -eq "v-$Version"
        } | ForEach-Object {
            Write-Warn2 "Clearing corrupt cache: $($_.FullName)"
            Remove-Item $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
        }
        # Partial download artifacts
        Get-ChildItem $root -Filter "*.tmp" -ErrorAction SilentlyContinue |
            Remove-Item -Force -ErrorAction SilentlyContinue
        Get-ChildItem $root -Filter "*$Version*.tar.gz*" -ErrorAction SilentlyContinue |
            Remove-Item -Force -ErrorAction SilentlyContinue
    }
}

function Set-VfoxTomlRustVersion {
    param([string]$Version)
    $toml = Join-Path $PSScriptRoot ".vfox.toml"
    $content = Get-Content -LiteralPath $toml -Raw
    $updated = [regex]::Replace(
        $content,
        '(?m)^(\s*rust\s*=\s*")([^"]+)("\s*)$',
        "`${1}$Version`${3}"
    )
    if ($updated -ne $content) {
        Set-Content -LiteralPath $toml -Value $updated -NoNewline
        Write-Info "Updated .vfox.toml rust = `"$Version`""
    }
}

function Install-VfoxSdkWithRetry {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Version,
        [int]$Attempts = 3
    )
    for ($i = 1; $i -le $Attempts; $i++) {
        if ((-not $ForceSetup) -and (Test-VfoxSdkInstalled -Name $Name -Version $Version)) {
            Write-Info "$Name@$Version already installed - skip download"
            return $true
        }
        Write-Step "Installing $Name@$Version (attempt $i/$Attempts) ..."
        $prev = $ErrorActionPreference
        $ErrorActionPreference = "Continue"
        $out = & vfox install "$Name@$Version" 2>&1 | Out-String
        $code = $LASTEXITCODE
        $ErrorActionPreference = $prev
        if ($out) { Write-Host $out }

        if ($code -eq 0 -or (Test-VfoxSdkInstalled -Name $Name -Version $Version)) {
            Write-Info "$Name@$Version installed"
            return $true
        }

        Write-Warn2 "$Name@$Version install failed (exit $code)"
        if ($Name -eq "rust") { Clear-VfoxRustCache -Version $Version }
        if ($i -lt $Attempts) {
            $delay = 3 * $i
            Write-Warn2 "Retrying in ${delay}s (network PROTOCOL_ERROR / stream errors are often transient) ..."
            Start-Sleep -Seconds $delay
        }
    }
    return $false
}

function Get-RustVersionCandidates {
    param([string]$Preferred)
    # Preferred pin first, then latest stable, then recent stables, then anything already cached/installed.
    # Rust has no formal LTS channel; "latest" (1.90.0) is the closest stable track.
    $ordered = [System.Collections.Generic.List[string]]::new()
    foreach ($v in @(
            $Preferred,
            "1.90.0",  # latest stable (vfox)
            "1.89.0",
            "1.88.0",
            "1.87.0",
            "1.85.1",
            "1.85.0"
        )) {
        if ($v -and -not $ordered.Contains($v)) { [void]$ordered.Add($v) }
    }
    foreach ($v in (Get-VfoxInstalledVersions -Name "rust")) {
        if (-not $ordered.Contains($v)) { [void]$ordered.Add($v) }
    }
    return @($ordered)
}

function Install-ProjectSdks {
    $tools = Get-VfoxTools
    Write-Step "Ensuring vfox plugins ..."
    if (-not (Test-VfoxPlugin "rust")) {
        Write-Step "Adding rust plugin (https://github.com/XZzYassin/vfox-rust) ..."
        & vfox add rust
        if ($LASTEXITCODE -ne 0) { throw "Failed to add rust plugin" }
    } else {
        Write-Info "rust plugin already installed - skip"
    }

    $resolved = @{}
    foreach ($name in $tools.Keys) {
        $ver = $tools[$name]
        if ($name -eq "rust") {
            $ok = $false
            $used = $null
            foreach ($candidate in (Get-RustVersionCandidates -Preferred $ver)) {
                if (Install-VfoxSdkWithRetry -Name "rust" -Version $candidate -Attempts 3) {
                    $ok = $true
                    $used = $candidate
                    break
                }
                Write-Warn2 "Giving up on rust@$candidate - trying next candidate ..."
            }
            if (-not $ok) {
                throw "Failed to install rust (tried pin + latest stable fallbacks). Check network / proxy and re-run."
            }
            if ($used -ne $ver) {
                Write-Warn2 "Pinned rust@$ver unavailable; using rust@$used instead"
                Set-VfoxTomlRustVersion -Version $used
            }
            $resolved[$name] = $used
            continue
        }

        if (-not (Install-VfoxSdkWithRetry -Name $name -Version $ver -Attempts 3)) {
            throw "Failed to install $name@$ver"
        }
        $resolved[$name] = $ver
    }

    Write-Step "Activating project-local SDK versions (.vfox/sdks) ..."
    foreach ($name in $resolved.Keys) {
        $ver = $resolved[$name]
        & vfox use -p "$name@$ver" 2>$null
        $bin = Join-Path $PSScriptRoot ".vfox\sdks\$name\bin"
        if (Test-Path $bin) { Add-PathFront $bin }
        $sdkLink = Join-Path $PSScriptRoot ".vfox\sdks\$name"
        if (Test-Path $sdkLink) {
            if ($name -eq "rust") {
                Merge-RustStdIntoSysroot -SdkRoot $sdkLink
            }
            $nested = Join-Path $sdkLink "bin"
            if (Test-Path $nested) { Add-PathFront $nested }
            Add-PathFront $sdkLink
        }
    }

    $sdksRoot = Join-Path $PSScriptRoot ".vfox\sdks"
    if (Test-Path $sdksRoot) {
        Get-ChildItem $sdksRoot -Directory | ForEach-Object {
            $b = Join-Path $_.FullName "bin"
            if (Test-Path $b) { Add-PathFront $b }
            Add-PathFront $_.FullName
            foreach ($sub in @("rustc\bin", "cargo\bin", "clippy-preview\bin", "rustfmt-preview\bin")) {
                $p = Join-Path $_.FullName $sub
                if (Test-Path $p) { Add-PathFront $p }
            }
            Get-ChildItem $_.FullName -Directory -ErrorAction SilentlyContinue | ForEach-Object {
                $innerBin = Join-Path $_.FullName "bin"
                if (Test-Path $innerBin) { Add-PathFront $innerBin }
            }
        }
    }

    $cargoHome = Join-Path $PSScriptRoot ".vfox\sdks\rust\cargo"
    if (Test-Path $cargoHome) {
        $env:CARGO_HOME = $cargoHome
    }

    if (-not (Test-Command rustc) -or -not (Test-Command cargo)) {
        throw "rustc/cargo not on PATH after vfox install. Check .vfox.toml and plugin layout."
    }
    Write-Info "rustc $(rustc --version)"
    Write-Info "cargo $(cargo --version)"
}

# ---------------------------------------------------------------------------
# 5. MSVC Build Tools (required by pc-windows-msvc Rust)
# ---------------------------------------------------------------------------
function Test-MsvcLinker {
    if (Test-Command link) { return $true }
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) { return $false }
    $install = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
    return [bool]$install
}

function Import-VsDevEnvironment {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) { return $false }
    $install = (& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null | Select-Object -First 1)
    if (-not $install) { return $false }
    $vsDevCmd = Join-Path $install "Common7\Tools\VsDevCmd.bat"
    if (-not (Test-Path $vsDevCmd)) { return $false }

    Write-Step "Importing VS developer environment ..."
    $cmd = "`"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && set"
    $vars = & cmd.exe /c $cmd
    foreach ($line in $vars) {
        if ($line -match "^(.*?)=(.*)$") {
            Set-Item -Path "Env:$($Matches[1])" -Value $Matches[2]
        }
    }
    return (Test-Command link)
}

function Ensure-MsvcBuildTools {
    if ((-not $ForceSetup) -and (Test-MsvcLinker)) {
        Write-Info "MSVC linker already available - skip Build Tools download"
        [void](Import-VsDevEnvironment)
        return
    }

    Write-Step "Installing Visual Studio 2022 Build Tools (MSVC) via winget ..."
    Write-Warn2 "This is large (~3-6 GB) and may take several minutes."
    $override = "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    & winget install --id $VsBuildToolsId --exact `
        --accept-source-agreements --accept-package-agreements `
        --override $override
    Refresh-Path

    if (-not (Import-VsDevEnvironment)) {
        throw "MSVC Build Tools installed but link.exe still not found. Reboot or open 'x64 Native Tools' and re-run."
    }
    Write-Info "MSVC linker ready: $((Get-Command link).Source)"
}

# ---------------------------------------------------------------------------
# 6. Build / run
# ---------------------------------------------------------------------------
function Stop-Tree {
    param([System.Diagnostics.Process]$Proc)
    if (-not $Proc) { return }
    try {
        if ($Proc.HasExited) { return }
        & taskkill /PID $Proc.Id /T /F 2>$null | Out-Null
    } catch { }
}

function Stop-IdeStack {
    if ($script:CleaningUp) { return }
    $script:CleaningUp = $true
    Write-Host ""
    Write-Step "Stopping Verilog IDE ..."
    Stop-Tree $script:AppProc
    $script:AppProc = $null
    Write-Info "Stopped."
}

function Assert-RustVersion {
    $tools = Get-VfoxTools
    $want = $tools["rust"]
    if (-not $want) { return }
    $out = (& rustc --version | Out-String).Trim()
    if ($out -notmatch [regex]::Escape($want)) {
        Write-Warn2 "Expected rustc $want from .vfox.toml but got: $out"
    } else {
        Write-Info "Pinned Rust OK: $out"
    }
}

function Invoke-CargoBuild {
    Assert-RustVersion
    [void](Import-VsDevEnvironment)

    $profileArgs = @()
    if ($Release) { $profileArgs = @("--release") }

    Write-Step ("cargo build {0}..." -f ($profileArgs -join " "))
    & cargo build @profileArgs
    if ($LASTEXITCODE -ne 0) { throw "cargo build failed" }
    Write-Info "Build succeeded"
}

function Start-IdeApp {
    Assert-RustVersion
    [void](Import-VsDevEnvironment)
    New-Item -ItemType Directory -Force -Path $script:IdeData | Out-Null

    if ($Build -or $Release) {
        Invoke-CargoBuild
    }

    $cargoArgs = @("run", "--quiet")
    if ($Release) { $cargoArgs = @("run", "--release", "--quiet") }

    if ($PrepOnly) {
        Write-Info "Prep-only - toolchain ready; not launching the IDE."
        $script:CleaningUp = $true
        return
    }

    Write-Step "Launching Verilog IDE (cargo $($cargoArgs -join ' ')) ..."
    Write-Info "Close the window or press Ctrl+C here to stop."
    $log = Join-Path $script:IdeData "app.log"

    $script:AppProc = Start-Process -FilePath "cargo" -ArgumentList $cargoArgs `
        -WorkingDirectory $PSScriptRoot -PassThru -NoNewWindow `
        -RedirectStandardOutput $log -RedirectStandardError $log

    try {
        while ($true) {
            if ($script:AppProc -and $script:AppProc.HasExited) {
                $code = $script:AppProc.ExitCode
                if ($code -ne 0) {
                    Write-Warn2 "IDE exited with code $code - last log lines:"
                    Get-Content $log -Tail 40 -ErrorAction SilentlyContinue | Write-Host
                }
                break
            }
            Start-Sleep -Seconds 1
        }
    } finally {
        Stop-IdeStack
    }
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
try {
    Ensure-Winget
    Ensure-Vfox
    Install-ProjectSdks
    Ensure-MsvcBuildTools

    if ($PrepOnly -and -not $Build -and -not $Release) {
        Write-Info "Prep complete (winget + vfox + rust $($(Get-VfoxTools)['rust']) + MSVC)."
        $script:CleaningUp = $true
        exit 0
    }

    if ($Build -and $PrepOnly) {
        Invoke-CargoBuild
        $script:CleaningUp = $true
        exit 0
    }

    Start-IdeApp
} catch {
    Write-Err2 $_.Exception.Message
    if (-not $PrepOnly) { Stop-IdeStack }
    exit 1
} finally {
    if (-not $PrepOnly) { Stop-IdeStack }
}
