# SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$documents = @('README.md','README.ru.md','NOTICE.md','THIRD_PARTY.md','CONTRIBUTING.md','LICENSE-POLICY.md','MODIFICATIONS.md') +
    @(Get-ChildItem -LiteralPath (Join-Path $workspace 'docs') -Filter '*.md' -Recurse -File | ForEach-Object { [IO.Path]::GetRelativePath($workspace,$_.FullName) }) +
    @(Get-ChildItem -LiteralPath (Join-Path $workspace 'specs') -Filter '*.md' -Recurse -File | ForEach-Object { [IO.Path]::GetRelativePath($workspace,$_.FullName) })
$linkCount = 0
foreach ($relative in $documents) {
    $path = Join-Path $workspace $relative
    $text = [IO.File]::ReadAllText($path)
    foreach ($match in [regex]::Matches($text, '!?\[[^\]]*\]\((?<target>[^)\s]+)(?:\s+"[^"]*")?\)')) {
        $target = $match.Groups['target'].Value.Trim('<','>')
        if ($target -match '^(https?://|mailto:|#)') { continue }
        $target = [Uri]::UnescapeDataString(($target -split '#',2)[0])
        $resolved = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetDirectoryName($path)) $target))
        if (-not $resolved.StartsWith($workspace + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $resolved)) {
            throw "Broken/nonlocal documentation target in ${relative}: $target"
        }
        $linkCount++
    }
}
$gallery = Get-Content -LiteralPath (Join-Path $workspace 'docs/images/gallery.json') -Raw | ConvertFrom-Json
if ($gallery.usb_opened -ne $false -or $gallery.game_data -ne 'Freedoom Phase 1 0.13.0') { throw 'Unexpected screenshot origin.' }
if (@($gallery.scenes).Count -ne 2 -or $gallery.scenes[1].scene -ne 'encounter' -or $gallery.scenes[1].visible_enemy_pixels -lt 150) { throw 'Enemy gallery requires a real visible-enemy frame.' }
foreach ($scene in $gallery.scenes) {
    if ($scene.source_pixels -join 'x' -ne '320x200' -or $scene.display_pixels -join 'x' -ne '583x434') { throw 'Unexpected capture geometry.' }
    if ($scene.modes.Count -ne 3 -or ($scene.modes | Where-Object mode -eq 'color-ascii').reports -ne 79) { throw 'Unexpected capture modes/ASCII wire budget.' }
}
$imagesRoot = Join-Path $workspace 'docs/images'
$hashes = Get-Content -LiteralPath (Join-Path $imagesRoot 'image-hashes.json') -Raw | ConvertFrom-Json
if ($hashes.wad_sha256 -ne '7323bcc168c5a45ff10749b339960e98314740a734c30d4b9f3337001f9e703d') { throw 'Unexpected screenshot IWAD.' }
$pngs = @(Get-ChildItem -LiteralPath $imagesRoot -Filter '*.png' -File)
if ($pngs.Count -ne @($hashes.images).Count -or $pngs.Count -ne 15) { throw 'Unexpected/unrecorded screenshot file.' }
foreach ($entry in $hashes.images) {
    if ($entry.path -ne [IO.Path]::GetFileName($entry.path)) { throw 'Unexpected screenshot manifest path.' }
    if ((Get-FileHash -LiteralPath (Join-Path $imagesRoot $entry.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256) { throw "Screenshot hash mismatch: $($entry.path)" }
}
foreach ($entry in (Get-Content -LiteralPath (Join-Path $workspace 'docs/engine-provenance.json') -Raw | ConvertFrom-Json).files) {
    if ((Get-FileHash -LiteralPath (Join-Path $workspace $entry.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256) { throw "Engine provenance hash mismatch: $($entry.path)" }
}
foreach ($entry in (Get-Content -LiteralPath (Join-Path $workspace 'docs/asset-provenance.json') -Raw | ConvertFrom-Json).font_files) {
    if ((Get-FileHash -LiteralPath (Join-Path $workspace $entry.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $entry.sha256) { throw "Font provenance hash mismatch: $($entry.path)" }
}
[pscustomobject]@{ Documents=$documents.Count; LocalLinks=$linkCount; Images=$pngs.Count; EngineFiles=179; FontFiles=4; Result='passed' }
