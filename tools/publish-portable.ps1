# Build the portable release: one NewMoonTracker.exe that runs on any 64-bit Windows 10/11 PC with nothing installed.
#
# The exe is the Rust build (crates\newmoon-tracker). It carries the page (app\ui), the decoded game tables
# (reference\*.tsv), the icon and the C runtime inside itself: nothing is unpacked anywhere, and the user's own data goes
# in a "data" folder made beside the exe. The window itself is Microsoft Edge WebView2, which comes with Windows 11 (and
# updated Windows 10); its loader is linked in, so there is no WebView2Loader.dll either.
# Pictures made locally from the game client (app\ui\*-icons) are never embedded.
#
# The GitHub release workflow (.github\workflows\release.yml) runs this same script. Needs Rust (rustup, stable,
# x86_64-pc-windows-msvc) and the Visual Studio C++ build tools.
#
#   pwsh -NoProfile -File tools\publish-portable.ps1                  # -> dist\NewMoonTracker.exe
#   pwsh -NoProfile -File tools\publish-portable.ps1 -Version 1.2.0   # stamp a version into the exe
param([string] $Version = '0.0.0', [string] $Out = 'dist')
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$outDir = if ([IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $root $Out }

if (Test-Path $outDir) { Remove-Item $outDir -Recurse -Force }
New-Item -ItemType Directory $outDir | Out-Null

# crates\newmoon-tracker\build.rs reads NEWMOON_VERSION for the exe's version resource; .cargo\config.toml links the
# C runtime statically (no VCRUNTIME140.dll needed on the user's PC)
$env:NEWMOON_VERSION = $Version
Push-Location $root
try {
  & cargo build --release --locked -p newmoon-tracker
  if ($LASTEXITCODE -ne 0) { throw "cargo build failed ($LASTEXITCODE)" }
} finally { Pop-Location; Remove-Item Env:NEWMOON_VERSION -ErrorAction SilentlyContinue }

$exe = Join-Path $outDir 'NewMoonTracker.exe'
Copy-Item (Join-Path $root 'target\release\NewMoonTracker.exe') $exe

# The release is one file: refuse an exe that still needs a runtime DLL beside it or on the user's PC
$bytes = [IO.File]::ReadAllBytes($exe)
$text = [Text.Encoding]::ASCII.GetString($bytes)
$needs = [regex]::Matches($text, '(?i)(vcruntime\d+|msvcp\d+|ucrtbase|api-ms-win-crt-[a-z0-9-]+|WebView2Loader)\.dll') | ForEach-Object Value | Sort-Object -Unique
if ($needs) { throw "The exe should need no runtime DLL, but it imports: $($needs -join ', ')" }
$extra = @(Get-ChildItem $outDir -File | Where-Object Name -ne 'NewMoonTracker.exe')
if ($extra) { throw "The release should be one exe, but the folder also has: $($extra.Name -join ', ')" }

$mb = [math]::Round((Get-Item $exe).Length / 1MB, 1)
Write-Host "Portable build: $exe ($mb MB, version $Version)"
