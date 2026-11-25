# Puts the "BABEL feedback" GitHub App's private key into the Worker as the secret GITHUB_APP_KEY.
# Run it yourself, with the .pem file GitHub downloaded ("Generate a private key" on the app's page):
#   pwsh -NoProfile -File feedback-worker\set-key.ps1 -KeyFile "$HOME\Downloads\<name>.private-key.pem"
# The key goes from the file straight to Cloudflare; it is never printed. GitHub can make a new key any time (and
# delete the old one) if this one is ever lost or leaked: then run this again with the new file.
param([Parameter(Mandatory)][string]$KeyFile)
$ErrorActionPreference = 'Stop'
# node.exe by its full path, and wrangler from this folder's node_modules (npm install): no npx, so a PowerShell window
# opened before Node.js was installed (its PATH has no Node) works too.
$node = @((Get-Command node.exe -CommandType Application -ErrorAction SilentlyContinue).Source) + "$env:ProgramFiles\nodejs\node.exe" |
    Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1
if (-not $node) { throw 'Node.js is not installed. Install it (winget install OpenJS.NodeJS.LTS), then run this again.' }
$wrangler = Join-Path $PSScriptRoot 'node_modules\wrangler\bin\wrangler.js'
if (-not (Test-Path -LiteralPath $wrangler)) { throw "wrangler is not installed in $PSScriptRoot. Run npm install there first." }
# wrangler.toml is git-ignored and kept in the main checkout only: a worktree uses that one
$main = (Split-Path $PSScriptRoot -Parent) -replace '[\\/]\.claude[\\/]worktrees[\\/][^\\/]+$', ''
$config = @((Join-Path $PSScriptRoot 'wrangler.toml'), (Join-Path $main 'feedback-worker\wrangler.toml')) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if (-not $config) { throw "No wrangler.toml here or in the main checkout's feedback-worker\ (see wrangler.example.toml)." }
if (-not (Test-Path -LiteralPath $KeyFile -PathType Leaf)) { throw "There is no file at $KeyFile." }
$pem = Get-Content -LiteralPath $KeyFile -Raw
if ($pem -notmatch '-----BEGIN (RSA )?PRIVATE KEY-----') { throw "$KeyFile is not a private key file (.pem)." }
Push-Location $PSScriptRoot
try {
    $pem | & $node $wrangler secret put GITHUB_APP_KEY --config $config
    if ($LASTEXITCODE -ne 0) { throw "wrangler could not save the secret (exit code $LASTEXITCODE)." }
} finally { Pop-Location }
Write-Host ''
Write-Host 'The key is saved in Cloudflare. Keep the .pem file somewhere safe, or delete it: GitHub can make a new one.'
