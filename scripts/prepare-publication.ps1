# SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param([string]$Destination = (Join-Path $PSScriptRoot '../dist'), [string]$Target = 'x86_64-pc-windows-msvc', [switch]$BundleDependencies)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'package-support.ps1')
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
& (Join-Path $PSScriptRoot 'validate-docs.ps1') | Out-Host
& (Join-Path $PSScriptRoot 'validate-licenses.ps1') -Target $Target | Out-Host
$reviewedInventory = Get-Content -LiteralPath (Join-Path $workspace 'docs/licensing/dependencies-windows.json') -Raw | ConvertFrom-Json
$rightsRegister = Get-Content -LiteralPath (Join-Path $workspace 'docs/licensing/review-register.json') -Raw | ConvertFrom-Json
$destinationRoot = [IO.Path]::GetFullPath($Destination)
$stage = Join-Path $destinationRoot ('mcdu-source-' + [DateTime]::UtcNow.ToString('yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N').Substring(0,8))
if (Test-Path -LiteralPath $stage) { throw 'Publication output must be a new directory.' }
New-Item -ItemType Directory -Path $stage -Force | Out-Null
Push-Location -LiteralPath $workspace
try {
    $allow = @('Cargo.toml','Cargo.lock','build.rs','LICENSE','LICENSES','LICENSE-POLICY.md','MODIFICATIONS.md','NOTICE.md','THIRD_PARTY.md','CONTRIBUTING.md','README.md','README.ru.md','run.ps1','probe-grid.ps1','rustfmt.toml','.editorconfig','.gitignore','.gitattributes','.github','src','tests','examples','vendor/doomgeneric','assets','scripts','docs','specs')
    foreach ($entry in $allow) {
        $source = Join-Path $workspace $entry
        if (-not (Test-Path -LiteralPath $source)) { throw "Required publication entry missing: $entry" }
        $copyTarget = Join-Path $stage $entry
        New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($copyTarget)) -Force | Out-Null
        Copy-Item -LiteralPath $source -Destination $copyTarget -Recurse -Force
    }
    $files = @(Get-ChildItem -LiteralPath $stage -Recurse -File -Force)
    foreach ($file in $files) {
        if ($file.Extension -in @('.wad','.exe','.dll','.pdb','.dsg','.cfg','.zip','.bin') -or
            $file.Name -like '.env*' -and $file.Name -ne '.env.example') {
            throw "Excluded runtime/binary/sensitive file in publication: $($file.FullName)"
        }
        if ($file.Extension -in @('.rs','.c','.h','.md','.toml','.json','.yml','.yaml','.ps1','.py','.svg','.txt','.sfd') -or $file.Name -in @('Cargo.lock')) {
            $body = [IO.File]::ReadAllText($file.FullName)
            # Pattern construction avoids matching the scanner's own source text.
            $patterns = @('AKIA' + '[A-Z0-9]{16}', 'gh[pousr]_' + '[A-Za-z0-9]{30,}', '-----BEGIN ' + '(?:RSA |EC |OPENSSH )?PRIVATE KEY-----')
            foreach ($pattern in $patterns) { if ($body -cmatch $pattern) { throw "Possible secret in $($file.Name); review manually. Value suppressed." } }
        }
    }
    $metadataText = & cargo metadata --offline --locked --format-version 1 --filter-platform $Target
    if ($LASTEXITCODE -ne 0) { throw 'Locked Cargo metadata failed. Fetch dependencies before preparing publication.' }
    $metadata = ($metadataText -join "`n") | ConvertFrom-Json
    $resolvedIds = @($metadata.resolve.nodes.id)
    $inventory = @()
    $otherTargets = @()
    $licenseRoot = Join-Path $stage 'THIRD_PARTY_LICENSES'
    New-Item -ItemType Directory -Path $licenseRoot -Force | Out-Null
    foreach ($package in ($metadata.packages | Sort-Object name,version)) {
        if ($null -eq $package.source) { continue }
        if ($package.id -notin $resolvedIds) {
            $otherTargets += [ordered]@{ name=$package.name; version=$package.version; declared_license=$package.license; repository=$package.repository; scope='metadata-only; not built for selected target' }
            continue
        }
        $reviewedPackage = @($reviewedInventory.packages | Where-Object { $_.name -eq $package.name -and $_.version -eq $package.version })
        if ($reviewedPackage.Count -ne 1) { throw "Dependency missing from reviewed inventory: $($package.name)" }
        $reviewedPackage = $reviewedPackage[0]
        $packageRoot = Split-Path -Parent $package.manifest_path
        $packageTarget = Join-Path $licenseRoot ($package.name + '-' + $package.version)
        New-Item -ItemType Directory -Path $packageTarget -Force | Out-Null
        foreach ($notice in $reviewedPackage.notices) {
            $noticeSource = [IO.Path]::GetFullPath((Join-Path $packageRoot $notice.path))
            $noticeTarget = [IO.Path]::GetFullPath((Join-Path $packageTarget $notice.path))
            if (-not $noticeSource.StartsWith($packageRoot + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase) -or
                -not $noticeTarget.StartsWith($packageTarget + [IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)) { throw 'Dependency notice path escapes package.' }
            New-Item -ItemType Directory -Path ([IO.Path]::GetDirectoryName($noticeTarget)) -Force | Out-Null
            Copy-Item -LiteralPath $noticeSource -Destination $noticeTarget -Force
            if ((Get-FileHash -LiteralPath $noticeTarget -Algorithm SHA256).Hash.ToLowerInvariant() -ne $notice.sha256) { throw 'Copied dependency notice differs from review.' }
        }
        $inventory += $reviewedPackage
    }
    if ($inventory.Count -ne $reviewedInventory.packages.Count) { throw 'Incomplete dependency notice inventory.' }
    $inventory | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $stage 'DEPENDENCIES.json') -Encoding utf8
    $otherTargets | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $stage 'OTHER-TARGET-DECLARATIONS.json') -Encoding utf8
    if ($BundleDependencies) {
        $vendorPath = Join-Path $stage 'dependencies'
        $config = (& cargo vendor --locked --offline --versioned-dirs $vendorPath) -join "`n"
        if ($LASTEXITCODE -ne 0) { throw 'Offline dependency vendoring failed; run cargo fetch --locked first.' }
        $config = $config.Replace($vendorPath.Replace('\','\\'), 'dependencies').Replace($vendorPath, 'dependencies')
        $config = $config.Replace($vendorPath.Replace('\','/'), 'dependencies')
        if ($config -notmatch 'directory = "dependencies"' -or $config -match '[A-Za-z]:[/\\]') { throw 'Vendored source config must be relocatable.' }
        New-Item -ItemType Directory -Path (Join-Path $stage '.cargo') -Force | Out-Null
        [IO.File]::WriteAllText((Join-Path $stage '.cargo/config.toml'), $config + "`n[net]`noffline = true`n")
    }
    $hashes = @(Get-ChildItem -LiteralPath $stage -Recurse -File -Force | Sort-Object FullName | ForEach-Object {
        [ordered]@{ path=[IO.Path]::GetRelativePath($stage,$_.FullName).Replace('\','/'); sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
    })
    [ordered]@{ kind=$(if ($BundleDependencies) {'source-with-dependencies'} else {'source-only'}); target=$Target; application_license='GPL-3.0-only'; license_policy='LICENSE-POLICY.md'; modification_record='MODIFICATIONS.md'; dependency_count=$inventory.Count; other_target_metadata_count=$otherTargets.Count; secret_scan='limited-pattern-check'; legal_clearance=$rightsRegister.legal_clearance; publication_status=$rightsRegister.publication_status; rights_review='docs/licensing/README.md'; review_register='docs/licensing/review-register.json'; open_rights_items=@($rightsRegister.items | Where-Object status -eq unresolved | ForEach-Object id); binary_corresponding_source=$(if ($BundleDependencies) {'dependency sources included; toolchain and system libraries external'} else {'not-assessed; dependency source is not bundled'}); files=$hashes } |
        ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $stage 'PUBLICATION-MANIFEST.json') -Encoding utf8
    $archive = $stage + '.zip'
    # ZipFile includes .github/.gitignore, unlike wildcard-based archive commands.
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    New-ReproducibleArchive -Directory $stage -Archive $archive
    & (Join-Path $PSScriptRoot 'verify-publication.ps1') -Archive $archive | Out-Host
    [pscustomobject]@{ Directory=$stage; Archive=$archive; Files=$hashes.Count; Dependencies=$inventory.Count; SHA256=(Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash; LegalClearance=$rightsRegister.legal_clearance }
} finally { Pop-Location }
