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
#   8. vfox (via winget) + tools from .vfox.toml (single pinned rust, download w/ progress)
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

function Write-DownloadProgressLine {
    param(
        [string]$Label,
        [long]$Received,
        [long]$Total,
        [long]$SpeedBytesPerSec,
        [ref]$LastPct
    )
    if ($Total -gt 0) {
        $pct = [int][Math]::Min(100, ($Received * 100) / $Total)
        if ($pct -eq $LastPct.Value -and ($Received % (512KB)) -ge 256KB) { return }
        $LastPct.Value = $pct
        $status = "{0} / {1}  ({2}/s)" -f (Format-ByteSize $Received), (Format-ByteSize $Total), (Format-ByteSize $SpeedBytesPerSec)
        Write-Progress -Activity "Downloading $Label" -Status $status -PercentComplete $pct
        $barWidth = 28
        $filled = [int](($barWidth * $pct) / 100)
        $bar = ("#" * $filled).PadRight($barWidth, "-")
        Write-Host ("`r  [{0}] {1,3}%  {2}" -f $bar, $pct, $status) -NoNewline
        try { [Console]::Out.Flush() } catch { }
    } else {
        $status = "{0} received  ({1}/s)" -f (Format-ByteSize $Received), (Format-ByteSize $SpeedBytesPerSec)
        Write-Progress -Activity "Downloading $Label" -Status $status -PercentComplete -1
        Write-Host ("`r  [---------- unknown size ----------]  {0}" -f $status) -NoNewline
        try { [Console]::Out.Flush() } catch { }
    }
}

function Save-UrlToFile {
    <#
    .SYNOPSIS
      Download a URL to disk with a live console progress bar (bytes + %).
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
    $request.Timeout = 1000 * 60 * 30
    $request.ReadWriteTimeout = 1000 * 60 * 30

    $response = $null
    $inStream = $null
    $outStream = $null
    try {
        $response = $request.GetResponse()
        $total = [long]$response.ContentLength
        if ($total -gt 0) {
            Write-Host ("  Size: {0}" -f (Format-ByteSize $total)) -ForegroundColor DarkGray
        }
        $inStream = $response.GetResponseStream()
        $outStream = [System.IO.File]::Create($OutFile)

        $buffer = New-Object byte[] (256KB)
        $received = [long]0
        $lastPct = -1
        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        $lastUi = [System.Diagnostics.Stopwatch]::StartNew()

        while ($true) {
            $read = $inStream.Read($buffer, 0, $buffer.Length)
            if ($read -le 0) { break }
            $outStream.Write($buffer, 0, $read)
            $received += $read

            # Refresh UI at least ~4x/sec so large downloads always look alive
            if ($lastUi.ElapsedMilliseconds -ge 250 -or $read -ge $buffer.Length) {
                $elapsedSec = [Math]::Max($sw.Elapsed.TotalSeconds, 0.001)
                $speed = [long]($received / $elapsedSec)
                Write-DownloadProgressLine -Label $Label -Received $received -Total $total `
                    -SpeedBytesPerSec $speed -LastPct ([ref]$lastPct)
                $lastUi.Restart()
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

function Get-VfoxRustCacheRoot {
    foreach ($root in @(
            (Join-Path $env:USERPROFILE ".vfox\cache\rust"),
            (Join-Path $env:USERPROFILE ".version-fox\cache\rust")
        )) {
        if (Test-Path (Split-Path $root -Parent)) { return $root }
    }
    return (Join-Path $env:USERPROFILE ".vfox\cache\rust")
}

function Clear-VfoxRustCache {
    param([string]$Version)
    $root = Get-VfoxRustCacheRoot
    if (-not (Test-Path $root)) { return }
    Get-ChildItem $root -ErrorAction SilentlyContinue | Where-Object {
        $_.Name -match [regex]::Escape($Version) -or $_.Name -eq "v-$Version"
    } | ForEach-Object {
        Write-Warn2 "Clearing corrupt cache: $($_.FullName)"
        Remove-Item $_.FullName -Recurse -Force -ErrorAction SilentlyContinue
    }
    Get-ChildItem $root -Filter "*.tmp" -ErrorAction SilentlyContinue |
        Remove-Item -Force -ErrorAction SilentlyContinue
    Get-ChildItem $root -Filter "*$Version*.tar.gz*" -ErrorAction SilentlyContinue |
        Remove-Item -Force -ErrorAction SilentlyContinue
}

function Get-RustDistTriple {
    $arch = switch ($env:PROCESSOR_ARCHITECTURE) {
        "ARM64" { "aarch64" }
        "x86"   { "i686" }
        default { "x86_64" }
    }
    return "$arch-pc-windows-msvc"
}

function Get-RustDistUrl {
    param([string]$Version)
    return ("https://static.rust-lang.org/dist/rust-{0}-{1}.tar.gz" -f $Version, (Get-RustDistTriple))
}

function Install-RustWithProgress {
    <#
    .SYNOPSIS
      Download the single pinned Rust stable with a live progress bar, unpack into
      the vfox cache layout, then let vfox register/use it. No version fallbacks.
    #>
    param(
        [Parameter(Mandatory = $true)][string]$Version,
        [int]$Attempts = 3
    )

    for ($i = 1; $i -le $Attempts; $i++) {
        if ((-not $ForceSetup) -and (Test-VfoxSdkInstalled -Name "rust" -Version $Version)) {
            Write-Info "rust@$Version already installed - skip download"
            return $true
        }

        Write-Step "Installing rust@$Version (attempt $i/$Attempts) - single stable pin ..."
        if ($ForceSetup) { Clear-VfoxRustCache -Version $Version }

        $url = Get-RustDistUrl -Version $Version
        $triple = Get-RustDistTriple
        $tmpRoot = Join-Path $env:TEMP "verilog-ide-rust-$Version"
        $tmpTar = Join-Path $tmpRoot "rust-$Version-$triple.tar.gz"
        $extractRoot = Join-Path $tmpRoot "extract"
        $cacheRoot = Get-VfoxRustCacheRoot
        $pkgDir = Join-Path $cacheRoot "v-$Version"
        $destDir = Join-Path $pkgDir "rust-$Version"

        try {
            if (Test-Path $tmpRoot) { Remove-Item $tmpRoot -Recurse -Force -ErrorAction SilentlyContinue }
            New-Item -ItemType Directory -Force -Path $tmpRoot | Out-Null

            Save-UrlToFile -Uri $url -OutFile $tmpTar -Label "Rust $Version ($triple)"

            Write-Step "Unpacking Rust $Version ..."
            Write-Host "  This can take a minute for a ~400 MB archive." -ForegroundColor DarkGray
            if (Test-Path $extractRoot) { Remove-Item $extractRoot -Recurse -Force }
            New-Item -ItemType Directory -Force -Path $extractRoot | Out-Null

            if (-not (Test-Command tar)) {
                throw "tar.exe is required to unpack Rust (built into Windows 10 1809+)."
            }
            & tar -xzf $tmpTar -C $extractRoot
            if ($LASTEXITCODE -ne 0) { throw "tar extract failed (exit $LASTEXITCODE)" }

            $inner = Get-ChildItem -LiteralPath $extractRoot -Directory -ErrorAction SilentlyContinue |
                Select-Object -First 1
            if (-not $inner) { throw "Unexpected Rust archive layout (no top-level folder)" }

            Write-Step "Installing into vfox cache: $destDir"
            if (Test-Path $pkgDir) { Remove-Item $pkgDir -Recurse -Force }
            New-Item -ItemType Directory -Force -Path $destDir | Out-Null
            Get-ChildItem -LiteralPath $inner.FullName -Force | ForEach-Object {
                Move-Item -LiteralPath $_.FullName -Destination $destDir -Force
            }

            # Prefer vfox install so metadata/hooks stay consistent; cache hit should be instant
            # if layout already matches. If vfox still tries a network fetch, PreInstall uses HTTPS
            # — we already have files, so CheckRuntimeExist should short-circuit.
            $prev = $ErrorActionPreference
            $ErrorActionPreference = "Continue"
            $out = & vfox install "rust@$Version" 2>&1 | Out-String
            $code = $LASTEXITCODE
            $ErrorActionPreference = $prev
            if ($out) { Write-Host $out }

            if ($code -eq 0 -or (Test-Path (Join-Path $destDir "rustc"))) {
                Write-Info "rust@$Version installed"
                return $true
            }

            Write-Warn2 "rust@$Version install failed (exit $code)"
        } catch {
            Write-Warn2 "rust@$Version install error: $($_.Exception.Message)"
        } finally {
            if (Test-Path $tmpRoot) {
                Remove-Item $tmpRoot -Recurse -Force -ErrorAction SilentlyContinue
            }
        }

        Clear-VfoxRustCache -Version $Version
        if ($i -lt $Attempts) {
            $delay = 3 * $i
            Write-Warn2 "Retrying in ${delay}s ..."
            Start-Sleep -Seconds $delay
        }
    }
    return $false
}

function Install-VfoxSdkWithRetry {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Version,
        [int]$Attempts = 3
    )
    if ($Name -eq "rust") {
        return (Install-RustWithProgress -Version $Version -Attempts $Attempts)
    }

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
        if ($i -lt $Attempts) {
            $delay = 3 * $i
            Write-Warn2 "Retrying in ${delay}s ..."
            Start-Sleep -Seconds $delay
        }
    }
    return $false
}

function Merge-RustStdIntoSysroot {
    param([string]$SdkRoot)
    if (-not $SdkRoot -or -not (Test-Path $SdkRoot)) { return }

    $rustcLib = Join-Path $SdkRoot "rustc\lib\rustlib"
    $candidates = @(
        (Get-ChildItem -Path $SdkRoot -Directory -Filter "rust-std-*" -ErrorAction SilentlyContinue)
    ) | Where-Object { $_ }

    foreach ($stdComp in $candidates) {
        $stdLib = Join-Path $stdComp.FullName "lib\rustlib"
        if (-not (Test-Path $stdLib)) { continue }
        $marker = Get-ChildItem -Path $stdLib -Directory -ErrorAction SilentlyContinue |
            Where-Object {
                $_.Name -like "*-pc-windows-msvc" -or
                $_.Name -like "*-unknown-linux-*" -or
                $_.Name -like "*-apple-darwin"
            } |
            Select-Object -First 1
        if ($marker -and (Test-Path (Join-Path $rustcLib $marker.Name))) {
            Write-Info "rust-std already merged into rustc sysroot ($($marker.Name))"
            return
        }
        Write-Step "Merging $($stdComp.Name) into rustc sysroot (vfox standalone layout) ..."
        New-Item -ItemType Directory -Force -Path $rustcLib | Out-Null
        Copy-Item -Path (Join-Path $stdLib "*") -Destination $rustcLib -Recurse -Force
        Write-Info "rust-std merged into $rustcLib"
        return
    }
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
            Write-Info "Using single pinned Rust stable: $ver (no version fallbacks)"
            if (-not (Install-VfoxSdkWithRetry -Name "rust" -Version $ver -Attempts 3)) {
                throw "Failed to install rust@$ver. Check network / DNS for static.rust-lang.org and re-run."
            }
            $resolved[$name] = $ver
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
