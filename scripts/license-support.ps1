# SPDX-License-Identifier: GPL-3.0-only
# Shared, read-only collection. Updating the checked-in review is a manual review step.
Set-StrictMode -Version Latest

function Get-DependencyLicenseInventory {
    param([Parameter(Mandatory)][string]$Workspace, [Parameter(Mandatory)][string]$Target)
    $choices = @{
        '0BSD OR MIT OR Apache-2.0' = 'MIT'
        'MIT OR Apache-2.0' = 'MIT'
        'Apache-2.0 OR MIT' = 'MIT'
        'MIT/Apache-2.0' = 'MIT'
        'Apache-2.0/MIT' = 'MIT'
        'MIT OR Zlib OR Apache-2.0' = 'MIT'
        'MIT OR Apache-2.0 OR Zlib' = 'MIT'
        'Zlib OR Apache-2.0 OR MIT' = 'MIT'
        'Unlicense OR MIT' = 'MIT'
        '(MIT OR Apache-2.0) AND Unicode-3.0' = 'MIT AND Unicode-3.0'
        'MIT' = 'MIT'
        'Zlib' = 'Zlib'
    }
    Push-Location -LiteralPath $Workspace
    try {
        $raw = & cargo metadata --offline --locked --format-version 1 --filter-platform $Target
        if ($LASTEXITCODE -ne 0) { throw 'Locked offline Cargo metadata failed; fetch the locked dependencies first.' }
        $graph = ($raw -join "`n") | ConvertFrom-Json
    } finally { Pop-Location }
    $lockText = [IO.File]::ReadAllText((Join-Path $Workspace 'Cargo.lock'))
    $lockChecksums = @{}
    foreach ($section in [regex]::Matches($lockText, '(?ms)^\[\[package\]\]\r?\n(?<body>.*?)(?=^\[\[package\]\]|\z)')) {
        $body = $section.Groups['body'].Value
        $lockName = [regex]::Match($body, '(?m)^name = "([^"]+)"').Groups[1].Value
        $lockVersion = [regex]::Match($body, '(?m)^version = "([^"]+)"').Groups[1].Value
        $lockHash = [regex]::Match($body, '(?m)^checksum = "([a-f0-9]{64})"').Groups[1].Value
        if ($lockHash) { $lockChecksums["$lockName/$lockVersion"] = $lockHash }
    }
    $inventory = @()
    foreach ($package in ($graph.packages | Sort-Object name,version)) {
        if ($package.id -notin $graph.resolve.nodes.id) { continue }
        if ($null -eq $package.source) {
            if ($package.id -ne $graph.resolve.root) { throw 'Unreviewed local/path dependency.' }
            continue
        }
        if (-not $package.source.StartsWith('registry+')) { throw "Unreviewed dependency source: $($package.name)" }
        if ($null -eq $package.license -or -not $choices.ContainsKey($package.license)) { throw "Unreviewed license declaration: $($package.name) $($package.license)" }
        $packageRoot = Split-Path -Parent $package.manifest_path
        $registryIndex = Split-Path -Leaf (Split-Path -Parent $packageRoot)
        $registryRoot = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $packageRoot))
        $crateArchive = Join-Path $registryRoot ("cache/$registryIndex/" + $package.name + '-' + $package.version + '.crate')
        if (-not (Test-Path -LiteralPath $crateArchive -PathType Leaf)) { throw "Missing cached registry archive: $($package.name). Fetch it before review." }
        $checksum = (Get-FileHash -LiteralPath $crateArchive -Algorithm SHA256).Hash.ToLowerInvariant()
        if (-not $lockChecksums.ContainsKey("$($package.name)/$($package.version)") -or $checksum -ne $lockChecksums["$($package.name)/$($package.version)"]) { throw "Registry package/lock checksum mismatch: $($package.name)" }
        $notices = @(Get-ChildItem -LiteralPath $packageRoot -Recurse -File | Where-Object { $_.Name -match '^(LICENSE|COPYING|NOTICE|UNLICENSE)' })
        if ($package.license_file) {
            $specific = [IO.Path]::GetFullPath((Join-Path $packageRoot $package.license_file))
            if (-not $specific.StartsWith($packageRoot + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Dependency license_file escapes package.' }
            if (-not (Test-Path -LiteralPath $specific -PathType Leaf)) { throw 'Declared license_file is missing.' }
            $notices += Get-Item -LiteralPath $specific
        }
        $notices = @($notices | Sort-Object FullName -Unique)
        if ($notices.Count -eq 0) { throw "Missing license text: $($package.name)" }
        $node = $graph.resolve.nodes | Where-Object id -eq $package.id
        $inventory += [ordered]@{
            name=$package.name; version=$package.version; source=$package.source
            registry_checksum=$checksum; declared_license=$package.license
            selected_package_license=$choices[$package.license]; features=@($node.features | Sort-Object)
            repository=$package.repository
            notices=@($notices | ForEach-Object { [ordered]@{
                path=[IO.Path]::GetRelativePath($packageRoot,$_.FullName).Replace('\','/')
                sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            } })
        }
    }
    [ordered]@{
        schema_version=1; review_date='2026-10-20'; target=$Target
        cargo_lock_sha256=(Get-FileHash -LiteralPath (Join-Path $Workspace 'Cargo.lock') -Algorithm SHA256).Hash.ToLowerInvariant()
        scope='Default-feature locked application graph; package declarations plus explicit native/file exceptions; no blanket file-level clearance'
        native_and_file_exceptions='docs/licensing/native-components.json'
        packages=$inventory
    }
}
