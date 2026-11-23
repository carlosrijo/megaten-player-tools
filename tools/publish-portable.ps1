# Build the portable release: one NewMoonTracker.exe that runs on any 64-bit Windows 10/11 PC with nothing installed.
#
# The exe carries the .NET runtime, the page (app\ui) and the decoded game tables (reference\*.tsv). On first start it
# unpacks them to %TEMP%\.net\NewMoonTracker\ and runs from there; the user's own data goes in a "data" folder made
# beside the exe. The window itself is Microsoft Edge WebView2, which comes with Windows 11 (and updated Windows 10).
# Pictures made locally from the game client (app\ui\*-icons) are never shipped.
#
# The GitHub release workflow (.github\workflows\release.yml) runs this same script.
#
#   pwsh -NoProfile -File tools\publish-portable.ps1                  # -> dist\NewMoonTracker.exe
#   pwsh -NoProfile -File tools\publish-portable.ps1 -Version 1.2.0   # stamp a version into the exe
param([string] $Version = '0.0.0', [string] $Out = 'dist')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$outDir = if ([IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $root $Out }

if (Test-Path $outDir) { Remove-Item $outDir -Recurse -Force }
& dotnet publish (Join-Path $root 'app') -c Release -r win-x64 --self-contained true -o $outDir --nologo `
  -p:PublishSingleFile=true `
  -p:IncludeNativeLibrariesForSelfExtract=true `
  -p:IncludeAllContentForSelfExtract=true `
  -p:EnableCompressionInSingleFile=true `
  -p:DebugType=none `
  -p:Version=$Version
if ($LASTEXITCODE -ne 0) { throw "dotnet publish failed ($LASTEXITCODE)" }

$exe = Join-Path $outDir 'NewMoonTracker.exe'
$extra = @(Get-ChildItem $outDir -File | Where-Object Name -ne 'NewMoonTracker.exe')
if ($extra) { throw "The release should be one exe, but publish also left: $($extra.Name -join ', ')" }
$mb = [math]::Round((Get-Item $exe).Length / 1MB, 1)
Write-Host "Portable build: $exe ($mb MB, version $Version)"
