# uninstall.ps1 - Clean Uninstaller for CopilotRemap
$ErrorActionPreference = "SilentlyContinue"

Write-Host "=========================================" -ForegroundColor Yellow
Write-Host "  Uninstalling Copilot Remap            " -ForegroundColor Yellow
Write-Host "=========================================" -ForegroundColor Yellow

# 1. Remove MSIX Package
$packages = Get-AppxPackage -Name "*CopilotRemap*"
foreach ($pkg in $packages) {
    Write-Host "Removing package: $($pkg.PackageFullName)..." -ForegroundColor Gray
    Remove-AppxPackage -Package $pkg.PackageFullName
}

# 2. Reset BrandedKey Registry if pointing to CopilotRemap
$regPath = "HKCU:\Software\Microsoft\Windows\Shell\BrandedKey"
$aumid = (Get-ItemProperty -Path $regPath -Name "AppAumid" -ErrorAction SilentlyContinue).AppAumid
if ($aumid -and $aumid.ToLower().Contains("copilotremap")) {
    Write-Host "Resetting Windows Copilot Key setting to Search..." -ForegroundColor Gray
    Set-ItemProperty -Path $regPath -Name "BrandedKeyChoiceType" -Value "Search"
    Set-ItemProperty -Path $regPath -Name "AppAumid" -Value ""
}

Write-Host "`nCopilotRemap uninstalled successfully." -ForegroundColor Green
