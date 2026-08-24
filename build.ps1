# build.ps1 - Automated Build, Packaging & Signing for CopilotRemap (Rust)
param (
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$projectRoot = $PSScriptRoot
Set-Location $projectRoot

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "  Building Copilot Remap (Rust / MSIX)  " -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan

# 1. Locate Windows SDK Tools
$sdkPaths = @(
    "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64",
    "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64",
    "C:\Program Files (x86)\Windows Kits\10\bin\x64"
)

$makeappx = $null
$signtool = $null

foreach ($p in $sdkPaths) {
    if (Test-Path (Join-Path $p "makeappx.exe")) {
        $makeappx = Join-Path $p "makeappx.exe"
        $signtool = Join-Path $p "signtool.exe"
        break
    }
}

if (-not $makeappx -or -not (Test-Path $makeappx)) {
    $found = Get-ChildItem -Path "C:\Program Files (x86)\Windows Kits" -Recurse -Filter "makeappx.exe" -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) {
        $makeappx = $found.FullName
        $signtool = Join-Path $found.DirectoryName "signtool.exe"
    }
}

if (-not $makeappx -or -not (Test-Path $makeappx)) {
    Write-Error "Could not locate makeappx.exe from Windows SDK. Please ensure Windows SDK 10/11 is installed."
}

Write-Host "Using makeappx: $makeappx" -ForegroundColor Gray
Write-Host "Using signtool: $signtool" -ForegroundColor Gray

# 2. Build Rust Release Binary
if (-not $SkipBuild) {
    Write-Host "`n[1/5] Compiling Rust release binary..." -ForegroundColor Yellow
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Cargo build failed."
    }
}

$exeSource = Join-Path $projectRoot "target\release\CopilotRemap.exe"
$settingsExeSource = Join-Path $projectRoot "target\release\Settings.exe"
if (-not (Test-Path $exeSource)) {
    Write-Error "Compiled executable not found at: $exeSource"
}
if (-not (Test-Path $settingsExeSource)) {
    Write-Error "Compiled executable not found at: $settingsExeSource"
}

# 3. Prepare MSIX Staging Directory
Write-Host "`n[2/5] Staging files for MSIX..." -ForegroundColor Yellow
$stagingDir = Join-Path $projectRoot "target\msix_staging"
if (Test-Path $stagingDir) { Remove-Item $stagingDir -Recurse -Force }
New-Item -ItemType Directory -Path $stagingDir -Force | Out-Null

Copy-Item $exeSource -Destination (Join-Path $stagingDir "CopilotRemap.exe") -Force
Copy-Item $settingsExeSource -Destination (Join-Path $stagingDir "Settings.exe") -Force
Copy-Item (Join-Path $projectRoot "packaging\AppxManifest.xml") -Destination (Join-Path $stagingDir "AppxManifest.xml") -Force
Copy-Item (Join-Path $projectRoot "packaging\Assets") -Destination (Join-Path $stagingDir "Assets") -Recurse -Force

# Create empty Public directory required by AppExtension
$publicDir = Join-Path $stagingDir "Public"
if (-not (Test-Path $publicDir)) { New-Item -ItemType Directory -Path $publicDir -Force | Out-Null }

# 4. Generate Self-Signed Certificate if needed
Write-Host "`n[3/5] Checking self-signed certificate..." -ForegroundColor Yellow
$certDir = Join-Path $projectRoot "packaging\certs"
if (-not (Test-Path $certDir)) { New-Item -ItemType Directory -Path $certDir -Force | Out-Null }

$pfxPath = Join-Path $certDir "CopilotRemap.pfx"
$cerPath = Join-Path $certDir "CopilotRemap.cer"
$certPassword = ConvertTo-SecureString -String "copilotremap" -Force -AsPlainText

if (-not (Test-Path $pfxPath) -or -not (Test-Path $cerPath)) {
    Write-Host "Creating new self-signed certificate with Publisher 'CN=CopilotRemapDev'..." -ForegroundColor Gray
    $cert = New-SelfSignedCertificate `
        -Type Custom `
        -Subject "CN=CopilotRemapDev" `
        -KeyUsage DigitalSignature `
        -FriendlyName "CopilotRemap Dev Certificate" `
        -CertStoreLocation "Cert:\CurrentUser\My" `
        -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3")

    Export-PfxCertificate -Cert $cert -FilePath $pfxPath -Password $certPassword | Out-Null
    Export-Certificate -Cert $cert -FilePath $cerPath | Out-Null
    Write-Host "Certificate exported to $pfxPath and $cerPath" -ForegroundColor Green
}

# 5. Pack MSIX
Write-Host "`n[4/5] Packaging MSIX..." -ForegroundColor Yellow
$msixOutput = Join-Path $projectRoot "target\CopilotRemap.msix"
if (Test-Path $msixOutput) { Remove-Item $msixOutput -Force }

& $makeappx pack /d $stagingDir /p $msixOutput /o
if ($LASTEXITCODE -ne 0) {
    Write-Error "makeappx packaging failed."
}

# 6. Sign MSIX
Write-Host "`n[5/5] Signing MSIX package..." -ForegroundColor Yellow
& $signtool sign /fd SHA256 /a /f $pfxPath /p "copilotremap" $msixOutput
if ($LASTEXITCODE -ne 0) {
    Write-Error "signtool signing failed."
}

$fileSize = (Get-Item $msixOutput).Length / 1KB
Write-Host "`n=========================================" -ForegroundColor Green
Write-Host "  Build & Signing Successful!            " -ForegroundColor Green
Write-Host "  Package: $msixOutput ($([Math]::Round($fileSize, 2)) KB)" -ForegroundColor Green
Write-Host "  Run .\install.ps1 to install & activate!" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Green
