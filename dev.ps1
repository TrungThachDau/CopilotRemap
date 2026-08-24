# E:\RustroverProjects\CopilotRemap\dev.ps1
# Instant Live Dev & Hot-Reload for Copilot Remap Settings UI

$Host.UI.RawUI.WindowTitle = "Copilot Remap - Hot Reload Dev"

Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Copilot Remap - Live Dev Hot-Reload           " -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host " [Status] Watching 'src/' for code changes..." -ForegroundColor Yellow
Write-Host " [Tip]    Press Ctrl+S in your editor to reload." -ForegroundColor Gray
Write-Host " [Exit]   Press Ctrl+C in this window to stop." -ForegroundColor Gray
Write-Host ""

$exePath = Join-Path $PSScriptRoot "target\debug\Settings.exe"
$srcPath = Join-Path $PSScriptRoot "src"
$process = $null

function Rebuild-And-Run {
    param($oldProcess)
    Write-Host "[Build] Compiling debug binary..." -ForegroundColor Gray
    $buildOutput = cargo build --bin Settings 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[Error] Compilation failed:" -ForegroundColor Red
        Write-Host $buildOutput -ForegroundColor Red
        return $oldProcess
    }

    if ($oldProcess -and -not $oldProcess.HasExited) {
        Write-Host "[Reload] Closing previous window..." -ForegroundColor DarkYellow
        Stop-Process -Id $oldProcess.Id -Force -ErrorAction SilentlyContinue
        Start-Sleep -Milliseconds 100
    }

    Write-Host "[Run] Launching Settings UI..." -ForegroundColor Green
    $p = Start-Process -FilePath $exePath -PassThru
    return $p
}

# Initial build & launch
$process = Rebuild-And-Run $null

# File system watcher for live reload
$watcher = New-Object System.IO.FileSystemWatcher
$watcher.Path = $srcPath
$watcher.Filter = "*.rs"
$watcher.IncludeSubdirectories = $true
$watcher.EnableRaisingEvents = $true

$lastEventTime = [DateTime]::MinValue

try {
    while ($true) {
        $change = $watcher.WaitForChanged([System.IO.WatcherChangeTypes]::Changed -bor [System.IO.WatcherChangeTypes]::Created, 300)
        if ($change.TimedOut -eq $false) {
            $now = [DateTime]::Now
            # 400ms debounce
            if (($now - $lastEventTime).TotalMilliseconds -gt 400) {
                $lastEventTime = $now
                Write-Host "`n[Change Detected] $($change.Name) -> Reloading UI..." -ForegroundColor Cyan
                $process = Rebuild-And-Run $process
            }
        }
    }
}
finally {
    $watcher.Dispose()
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
    Write-Host "`n[Dev Mode Stopped]" -ForegroundColor Yellow
}
