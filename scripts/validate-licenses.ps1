# SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param([string]$Target = 'x86_64-pc-windows-msvc')
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
. (Join-Path $PSScriptRoot 'license-support.ps1')
$expected = Get-Content -LiteralPath (Join-Path $workspace 'docs/licensing/dependencies-windows.json') -Raw | ConvertFrom-Json
if ($Target -ne $expected.target) { throw 'This target has no checked-in license review. Review it before publication.' }
$current = Get-DependencyLicenseInventory -Workspace $workspace -Target $Target
if (($current | ConvertTo-Json -Depth 30 -Compress) -cne ($expected | ConvertTo-Json -Depth 30 -Compress)) { throw 'License inventory changed. Review Cargo.lock, selected features, declarations and nested notices; do not blindly regenerate the reviewed inventory.' }

foreach ($file in (Get-Content -LiteralPath (Join-Path $workspace 'docs/licensing/license-texts.json') -Raw | ConvertFrom-Json).files) {
    $path = [IO.Path]::GetFullPath((Join-Path $workspace $file.path))
    if (-not $path.StartsWith($workspace + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) { throw 'License-text path escapes workspace.' }
    if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.sha256) { throw "License/copyright text changed: $($file.path)" }
}
$metadataRaw = & cargo metadata --offline --locked --format-version 1 --filter-platform $Target --manifest-path (Join-Path $workspace 'Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'Cargo metadata failed while checking native exceptions.' }
$metadata = ($metadataRaw -join "`n") | ConvertFrom-Json
foreach ($component in (Get-Content -LiteralPath (Join-Path $workspace 'docs/licensing/native-components.json') -Raw | ConvertFrom-Json).components) {
    $package = @($metadata.packages | Where-Object { $_.name -eq $component.package -and $_.version -eq $component.version })
    if ($package.Count -ne 1) { throw 'Native component package changed; review required.' }
    $packageRoot = Split-Path -Parent $package[0].manifest_path
    foreach ($file in $component.reviewed_files) {
        $path = [IO.Path]::GetFullPath((Join-Path $packageRoot $file.path))
        if (-not $path.StartsWith($packageRoot + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) { throw 'Native review path escapes package.' }
        if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.sha256) { throw "Reviewed native/file exception changed: $($component.package)/$($file.path)" }
    }
}
$engine = Get-Content -LiteralPath (Join-Path $workspace 'docs/engine-provenance.json') -Raw | ConvertFrom-Json
$enginePaths = @($engine.files.path | Sort-Object)
$actualEnginePaths = @(Get-ChildItem -LiteralPath (Join-Path $workspace 'vendor/doomgeneric') -File | ForEach-Object { [IO.Path]::GetRelativePath($workspace,$_.FullName).Replace('\','/') } | Sort-Object)
if (($enginePaths -join "`n") -cne ($actualEnginePaths -join "`n")) { throw 'Unrecorded or missing vendored engine file.' }
foreach ($file in $engine.files) {
    if ((Get-FileHash -LiteralPath (Join-Path $workspace $file.path) -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.sha256) { throw "Engine changed since provenance review: $($file.path)" }
}
$register = Get-Content -LiteralPath (Join-Path $workspace 'docs/licensing/review-register.json') -Raw | ConvertFrom-Json
if ($register.legal_clearance -ne $false) { throw 'This checker cannot certify legal clearance.' }
foreach ($item in $register.items) {
    foreach ($evidence in $item.evidence) {
        $evidencePath = [IO.Path]::GetFullPath((Join-Path $workspace $evidence))
        if (-not $evidencePath.StartsWith($workspace + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $evidencePath -PathType Leaf)) { throw "Missing/nonlocal review evidence: $evidence" }
    }
}
$unresolved = @($register.items | Where-Object status -eq unresolved)
if ($unresolved.Count -gt 0) { Write-Warning ('Rights review remains unresolved: ' + ($unresolved.id -join ', ') + '. Technical pass is not permission to publish.') }
[pscustomobject]@{ TechnicalChecks='passed'; Packages=$current.packages.Count; EngineFiles=$engine.files.Count; UnresolvedRights=$unresolved.Count; LegalClearance=$false }
