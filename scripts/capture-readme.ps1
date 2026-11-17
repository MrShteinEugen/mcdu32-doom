# SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param([string]$Out = 'docs/images')
$ErrorActionPreference = 'Stop'
Push-Location -LiteralPath (Join-Path $PSScriptRoot '..')
try {
    $wad = 'data/freedoom-0.13.0/freedoom1.wad'
    if (-not (Test-Path -LiteralPath $wad)) { throw 'Run scripts/setup-freedoom.ps1 first.' }
    $expected = '7323bcc168c5a45ff10749b339960e98314740a734c30d4b9f3337001f9e703d'
    $actual = (Get-FileHash -LiteralPath $wad -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) { throw 'Gallery requires the pinned Freedoom 0.13.0 Phase 1 IWAD, not commercial DOOM data.' }
    $workspace = (Get-Location).Path
    $outputRoot = [IO.Path]::GetFullPath((Join-Path $workspace $Out))
    if (-not $outputRoot.StartsWith($workspace + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Gallery output must be inside this workspace.' }
    $staging = Join-Path $workspace ('output/readme-gallery-' + [guid]::NewGuid().ToString('N'))
    & cargo run --release --locked --example readme_gallery -- --out $staging
    if ($LASTEXITCODE -ne 0) { throw 'Screenshot capture failed.' }
    New-Item -ItemType Directory -Path $outputRoot -Force | Out-Null
    Get-ChildItem -LiteralPath $staging -File | Copy-Item -Destination $outputRoot -Force
    $pictures = @(Get-ChildItem -LiteralPath $outputRoot -Filter '*.png' -File | Sort-Object Name | ForEach-Object {
        [ordered]@{path=$_.Name;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()}
    })
    [ordered]@{game_data='Freedoom Phase 1 0.13.0';wad_sha256=$actual;images=$pictures} |
        ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $outputRoot 'image-hashes.json') -Encoding utf8
} finally { Pop-Location }
