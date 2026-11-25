# Opens the diagnostics files attached to a feedback issue (the id after "Diagnostics attached" in the issue):
#   pwsh -NoProfile -File feedback-worker\open-files.ps1 <id>
# Downloads them from the feedback service's store, decrypts them with .secrets\feedback-files-private.pem, puts them in
# .secrets\feedback-files\<id>\ (git-ignored) and shows that folder. Needs wrangler signed in (npx wrangler login).
param([Parameter(Mandatory)][string]$Id)
$ErrorActionPreference = 'Stop'
$node = @((Get-Command node.exe -CommandType Application -ErrorAction SilentlyContinue).Source) + "$env:ProgramFiles\nodejs\node.exe" |
    Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
if (-not $node) { throw 'Node.js is not installed. Install it (winget install OpenJS.NodeJS.LTS), then run this again.' }
$out = & $node (Join-Path $PSScriptRoot 'tools\open-files.mjs') $Id
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$out
$dir = ($out | Select-Object -Last 1) -replace '^\d+ files in ', ''
if (Test-Path -LiteralPath $dir) { Start-Process explorer.exe "`"$dir`"" }
