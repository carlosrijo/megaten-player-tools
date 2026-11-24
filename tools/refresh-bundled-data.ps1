# Copies the server data the reader needs into reference\bundled\, which is compiled into the exe. A player with only
# the exe has no server or website folders, so without these copies their numbers come out short (on 2026-11-24 a
# snapshot without them found 88 bonuses instead of 140). When the folders are set in data\config.json, the files on
# disk still win; these copies are only the fallback.
#
# Run it again when the server's tokusei, constants or the website's armory data change, then rebuild.
#
#   tools\refresh-bundled-data.ps1 [-TokuseiXml <dir>] [-Constants <file>] [-Website <content dir>]
param(
    # the datastore repo (same files as the deployed server's, plus the LICENSE that has to go along)
    [string]$TokuseiXml = (Join-Path (Split-Path (Split-Path $PSScriptRoot)) 'comp_hack\datastore\data\tokusei'),
    [string]$Constants = "$env:USERPROFILE\smt\deploy\data\config\constants.xml",
    [string]$Website = (Join-Path (Split-Path (Split-Path $PSScriptRoot)) 'ai-custom-smt-server\website\content')
)
$ErrorActionPreference = 'Stop'
$out = Join-Path (Split-Path $PSScriptRoot) 'reference\bundled'
foreach ($p in $TokuseiXml, $Constants, $Website) {
    if (-not (Test-Path $p)) { throw "Not found: $p" }
}

if (Test-Path $out) { Remove-Item -Recurse -Force $out }
New-Item -ItemType Directory -Force "$out\tokusei", "$out\website\armory", "$out\website\wiki" | Out-Null

# comp_hack's tokusei XML: what every effect does (the client only has the text). CC BY-SA 4.0: its license goes along.
Copy-Item "$TokuseiXml\*.xml" "$out\tokusei\"
$license = Join-Path (Split-Path (Split-Path $TokuseiXml)) 'LICENSE'
if (Test-Path $license) { Copy-Item $license "$out\tokusei\LICENSE" }
# constants.xml: digitalize rates, Mitama set boost, quest and compendium bonuses, clan level skills
Copy-Item $Constants "$out\constants.xml"

# The website's files whose data no game table in reference\ has yet. Sub-categories and special effects are left
# out: reference\items-game.tsv and sitem-game.tsv have them all.
foreach ($f in 'armory\tokusei-effects.json', 'armory\passive-skills.json', 'armory\enchant-sets.json',
               'armory\mod-effects.json', 'armory\expertises.json', 'armory\expertise-rank-skills.json',
               'wiki\enchants.json') {
    Copy-Item "$Website\$f" "$out\website\$f"
}
# devils.json is 4 MB; the reader takes only the demons' Law/Chaos points from it
$devils = Get-Content "$Website\armory\devils.json" -Raw | ConvertFrom-Json
[ordered]@{ source = $devils.source; generatedAt = $devils.generatedAt; lncs = $devils.lncs } |
    ConvertTo-Json -Depth 5 -Compress | Set-Content "$out\website\armory\devils.json" -NoNewline -Encoding utf8NoBOM

# items.json (11 MB) and skills.json (5 MB): only the fields the reader reads. They give the English names of the items
# and skills the client still has in Japanese (e.g. 裁きの魔冠 is Judgment Demon Crown), and the icons the client's
# tables lack.
function Write-Trimmed($file, $list, [string[]]$keep) {
    $j = Get-Content "$Website\$file" -Raw | ConvertFrom-Json
    $rows = foreach ($r in $j.$list) {
        $o = [ordered]@{}
        foreach ($k in $keep) { if ($null -ne $r.$k) { $o[$k] = $r.$k } }
        $o
    }
    [ordered]@{ source = $j.source; generatedAt = $j.generatedAt; $list = @($rows) } |
        ConvertTo-Json -Depth 6 -Compress | Set-Content "$out\website\$file" -NoNewline -Encoding utf8NoBOM
}
Write-Trimmed 'wiki\items.json' 'items' 'id', 'name', 'icon', 'iconAsset', 'gender', 'equipType', 'weaponType', 'basicFeatures', 'characteristics'
Write-Trimmed 'wiki\skills.json' 'skills' 'id', 'name', 'iconAsset'

@"
# Bundled server data

Copies compiled into the reader, so a player with only the exe gets the same numbers as one with the server's folders.
Made by ``tools\refresh-bundled-data.ps1`` on $(Get-Date -Format yyyy-MM-dd); do not edit by hand. The folders set in
``data\config.json`` (websiteContent, tokuseiXml, serverConstants) win when they exist.

| Here | From |
|---|---|
| ``tokusei\*.xml`` | comp_hack's datastore, ``data\tokusei`` ([colpertac/datastore](https://github.com/colpertac/datastore)), CC BY-SA 4.0 (``tokusei\LICENSE``) |
| ``constants.xml`` | the IMAGINE server's ``config\constants.xml`` (also in [colpertac/ai-custom-smt-server](https://github.com/colpertac/ai-custom-smt-server), ``deploy\seed\config``) |
| ``website\`` | the IMAGINE website's ``content`` folder ([colpertac/ai-custom-smt-server](https://github.com/colpertac/ai-custom-smt-server)); ``items.json``, ``skills.json`` and ``devils.json`` keep only the fields the reader reads |
"@ | Set-Content "$out\README.md" -Encoding utf8NoBOM

$files = Get-ChildItem $out -Recurse -File
'{0} files, {1:N1} MB in {2}' -f $files.Count, (($files | Measure-Object Length -Sum).Sum / 1MB), $out
