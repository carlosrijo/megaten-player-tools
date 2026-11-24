# Close the app, build it, open it again: one command, so a build never leaves the old program behind.
#
# Windows locks a running program, so "cargo build" with BABEL open cannot replace BABEL.exe
# (the page files are inside the exe, so nothing changes until it is replaced). This closes the app first (only the copy
# the desktop shortcut runs, target\release\BABEL.exe, never a --capture run from elsewhere), builds the reader
# (newmoon-reader.exe) and the app, and starts the app again, also when the build failed (with what was there before).
#
#   pwsh -NoProfile -File tools\rebuild-app.ps1            # close, build, reopen
#   pwsh -NoProfile -File tools\rebuild-app.ps1 -NoStart   # close and build only
param([switch] $NoStart)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'target\release\BABEL.exe'

# 1. close the app the shortcut runs: ask its window to close, and stop it only if it hasn't after a few seconds
$running = @(Get-Process BABEL -ErrorAction SilentlyContinue | Where-Object { $_.Path -and ($_.Path -ieq $exe) })
foreach ($p in $running) {
  Write-Host "Closing BABEL (process $($p.Id))..."
  $null = $p.CloseMainWindow()
  if (-not $p.WaitForExit(5000)) { Stop-Process -Id $p.Id -Force; $null = $p.WaitForExit(5000) }
}

# 2. build the reader (the command line tool) and the app (which embeds the page files): both exes in target\release
$ok = $true
Push-Location $root
try {
  Write-Host "Building the reader and the app..."
  $out = & cargo build --release --workspace 2>&1
  if ($LASTEXITCODE -ne 0) { $ok = $false; $out | Select-Object -Last 25 | ForEach-Object { Write-Host $_ }; Write-Host "Build failed" -ForegroundColor Red }
} finally { Pop-Location }
if ($ok) { Write-Host "Built: $((Get-Item $exe).LastWriteTime.ToString('HH:mm:ss'))" -ForegroundColor Green }

# 3. open it again (the new build, or the old one when the build failed)
if (-not $NoStart -and (Test-Path $exe)) {
  Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe)
  Write-Host "BABEL is open again$(if (-not $ok) { ' (the previous build: this one failed)' })."
}
if (-not $ok) { exit 1 }
