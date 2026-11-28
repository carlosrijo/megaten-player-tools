# Builds the BABEL website: the app's page (app\ui) in web mode, in one folder, for Cloudflare Pages
# (.github\workflows\release.yml, job "website").
#
#   pwsh tools\build-site.ps1 [-Out site]
#
# What goes online: only the files git tracks in app\ui, without the game art made locally (*-icons) and anything made
# with the sealed data (*.local.json). The copy of index.html is marked as the website, so app.js runs in web mode (WEB).
param([string]$Out = 'site')
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$dest = if ([IO.Path]::IsPathRooted($Out)) { $Out } else { Join-Path $root $Out }
if (Test-Path $dest) { Remove-Item -Recurse -Force $dest }
New-Item -ItemType Directory -Force $dest | Out-Null

# Only what git tracks: nothing made on this PC can slip in.
$files = @(git -C $root ls-files -- app/ui)
if ($LASTEXITCODE -ne 0 -or -not $files) { throw 'git could not list app\ui.' }
$copied = 0
foreach ($f in $files) {
  if ($f -match '(^|/)[^/]*-icons/' -or $f -match '\.local\.json$') { continue }
  $to = Join-Path $dest $f.Substring('app/ui/'.Length)
  New-Item -ItemType Directory -Force (Split-Path -Parent $to) | Out-Null
  Copy-Item -LiteralPath (Join-Path $root $f) -Destination $to
  $copied++
}

# The website marker (app.js: WEB).
$index = Join-Path $dest 'index.html'
$html = [IO.File]::ReadAllText($index)
$charset = '<meta charset="utf-8">'
if (-not $html.Contains($charset)) { throw "index.html has no $charset line to put the website marker after." }
$html = $html.Replace($charset, $charset + "`r`n  <meta name=`"babel-mode`" content=`"web`">")
[IO.File]::WriteAllText($index, $html)

# Check again before anything goes online.
$bad = @(Get-ChildItem -Recurse -File $dest | Where-Object { $_.Name -like '*.local.json' -or $_.FullName -match '-icons[\\/]' -or $_.Name -like '*.key' })
if ($bad) { throw "Not for the website: $($bad.FullName -join ', ')" }
$sealed = @(Get-ChildItem -Recurse -File $dest -Include *.json, *.xml, *.tsv | Where-Object { Select-String -LiteralPath $_.FullName -Pattern 'name="Tokusei"' -SimpleMatch -Quiet })
if ($sealed) { throw "Tokusei XML in: $($sealed.FullName -join ', ')" }

$mb = [Math]::Round(((Get-ChildItem -Recurse -File $dest | Measure-Object Length -Sum).Sum) / 1MB, 1)
Write-Host "Built the website in $dest ($copied files, $mb MB)."
