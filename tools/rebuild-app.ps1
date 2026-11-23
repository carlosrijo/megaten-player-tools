# Close the app, build it, open it again: one command, so a build never leaves the old program behind.
#
# Windows locks a running program's files, so "dotnet build app" with NewMoon Build Tracker open copies the new page
# files but cannot replace NewMoonTracker.exe / .dll (error MSB3027): the app then runs the new page on the old program.
# This closes the app first (only the copy the desktop shortcut runs, never a --capture run from elsewhere), builds the
# reader and the app, and starts the app again, also when the build failed (with what was there before).
#
#   pwsh -NoProfile -File tools\rebuild-app.ps1            # close, build, reopen
#   pwsh -NoProfile -File tools\rebuild-app.ps1 -NoStart   # close and build only
param([switch] $NoStart)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$exe = Join-Path $root 'app\bin\Release\net10.0-windows\NewMoonTracker.exe'

# 1. close the app the shortcut runs: ask its window to close, and stop it only if it hasn't after a few seconds
$running = @(Get-Process NewMoonTracker -ErrorAction SilentlyContinue | Where-Object { $_.Path -and ($_.Path -ieq $exe) })
foreach ($p in $running) {
  Write-Host "Closing NewMoon Build Tracker (process $($p.Id))..."
  $null = $p.CloseMainWindow()
  if (-not $p.WaitForExit(5000)) { Stop-Process -Id $p.Id -Force; $null = $p.WaitForExit(5000) }
}

# 2. build the reader (the command line tool) and the app (which takes the reader and the page files with it)
$ok = $true
Push-Location $root
try {
  foreach ($proj in '.', 'app') {
    Write-Host "Building $(if ($proj -eq '.') { 'the reader' } else { 'the app' })..."
    $out = & dotnet build $proj -c Release --nologo -v q 2>&1
    if ($LASTEXITCODE -ne 0) { $ok = $false; $out | Select-Object -Last 15 | ForEach-Object { Write-Host $_ }; Write-Host "Build failed: $proj" -ForegroundColor Red; break }
  }
} finally { Pop-Location }
if ($ok) { Write-Host "Built: $((Get-Item $exe).LastWriteTime.ToString('HH:mm:ss'))" -ForegroundColor Green }

# 3. open it again (the new build, or the old one when the build failed)
if (-not $NoStart -and (Test-Path $exe)) {
  Start-Process -FilePath $exe -WorkingDirectory (Split-Path $exe)
  Write-Host "NewMoon Build Tracker is open again$(if (-not $ok) { ' (the previous build: this one failed)' })."
}
if (-not $ok) { exit 1 }
