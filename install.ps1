# install.ps1 - Automated 1-Click Installer for CopilotRemap
param (
    [switch]$BuildFirst,
    [switch]$SkipSettings
)

$ErrorActionPreference = "Stop"
$projectRoot = $PSScriptRoot
Set-Location $projectRoot

# Self-elevation check
$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = New-Object Security.Principal.WindowsPrincipal($identity)
$isAdmin = $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "Elevating permissions to install certificate and package..." -ForegroundColor Yellow
    $argsList = "-ExecutionPolicy Bypass -NoProfile -File `"$PSCommandPath`""
    if ($BuildFirst) { $argsList += " -BuildFirst" }
    if ($SkipSettings) { $argsList += " -SkipSettings" }
    
    Start-Process powershell.exe -Verb RunAs -ArgumentList $argsList
    exit
}

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  Installing Copilot Remap (Rust)       " -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

$msixPath = Join-Path $projectRoot "target\CopilotRemap.msix"
$cerPath = Join-Path $projectRoot "packaging\certs\CopilotRemap.cer"

if ($BuildFirst -or -not (Test-Path $msixPath) -or -not (Test-Path $cerPath)) {
    Write-Host "`n[1/3] Building & Packaging..." -ForegroundColor Yellow
    & (Join-Path $projectRoot "build.ps1")
}

# 1. Trust Certificate in LocalMachine\TrustedPeople (Elevated)
Write-Host "`n[1/3] Trusting self-signed certificate in LocalMachine\TrustedPeople..." -ForegroundColor Yellow
try {
    Import-Certificate -FilePath $cerPath -CertStoreLocation "Cert:\LocalMachine\TrustedPeople" | Out-Null
    Write-Host "Certificate trusted in LocalMachine\TrustedPeople successfully." -ForegroundColor Green
} catch {
    Write-Warning "Could not import to LocalMachine\TrustedPeople: $_"
}

# 2. Install MSIX Package
Write-Host "`n[2/3] Installing MSIX package..." -ForegroundColor Yellow

# If old package is installed, remove it cleanly first
$oldPkg = Get-AppxPackage -Name "*CopilotRemap*" -ErrorAction SilentlyContinue
if ($oldPkg) {
    Write-Host "Removing existing version ($($oldPkg.Version))..." -ForegroundColor Gray
    Remove-AppxPackage -Package $oldPkg.PackageFullName -ErrorAction SilentlyContinue
}

Add-AppxPackage -Path $msixPath -ForceApplicationShutdown
Write-Host "MSIX package installed successfully!" -ForegroundColor Green

# 3. Retrieve Package AUMID & Configure Windows Copilot Key Registry
Write-Host "`n[3/3] Activating Copilot key provider in Windows..." -ForegroundColor Yellow
$pkg = Get-AppxPackage -Name "*CopilotRemap*" | Select-Object -First 1
if ($pkg) {
    $aumid = "$($pkg.PackageFamilyName)!CopilotRemap"
    Write-Host "Package Family Name: $($pkg.PackageFamilyName)" -ForegroundColor Gray
    Write-Host "Configuring AUMID: $aumid" -ForegroundColor Gray

    $regPath = "HKCU:\Software\Microsoft\Windows\Shell\BrandedKey"
    if (-not (Test-Path $regPath)) {
        New-Item -Path $regPath -Force | Out-Null
    }

    Set-ItemProperty -Path $regPath -Name "BrandedKeyChoiceType" -Value "App" -Force
    Set-ItemProperty -Path $regPath -Name "AppAumid" -Value $aumid -Force

    Write-Host "Copilot Key is now registered to CopilotRemap in Windows Registry!" -ForegroundColor Green
} else {
    Write-Warning "Could not find installed package details to auto-register AUMID."
}

Write-Host "`n=========================================" -ForegroundColor Green
Write-Host "  Installation Completed!                " -ForegroundColor Green
Write-Host "  You can now press the physical Copilot " -ForegroundColor Green
Write-Host "  key or Win+C to trigger your remap!    " -ForegroundColor Green
Write-Host "=========================================" -ForegroundColor Green

if (-not $SkipSettings) {
    Write-Host "`nLaunching Settings UI..." -ForegroundColor Cyan
    Start-Process -FilePath (Join-Path $projectRoot "target\release\Settings.exe")
}

Start-Sleep -Seconds 3
