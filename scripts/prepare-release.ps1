# SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param([string]$Destination = (Join-Path $PSScriptRoot '../dist'), [switch]$RequireClean)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'package-support.ps1')
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
if (-not $IsWindows) { throw 'Windows/MSVC release packaging must run on Windows.' }
Push-Location -LiteralPath $workspace
try {
    $dirty = @(& git status --porcelain --untracked-files=all)
    if ($LASTEXITCODE -ne 0) { throw 'Git checkout required to record build identity.' }
    if ($RequireClean -and $dirty.Count) { throw 'Release preparation requires a clean Git checkout, including new source files.' }
    $commit = (& git rev-parse HEAD).Trim()
    $metadataText = & cargo metadata --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Fetch locked dependencies before packaging.' }
    $metadata = ($metadataText -join "`n") | ConvertFrom-Json
    $version = ($metadata.packages | Where-Object id -eq $metadata.resolve.root).version
    if ($env:GITHUB_REF_TYPE -eq 'tag' -and $env:GITHUB_REF_NAME -ne "v$version") { throw 'Tag must match Cargo.toml: v<version>.' }
    $release = Join-Path ([IO.Path]::GetFullPath($Destination)) ("mcdu-doom-$version-" + [guid]::NewGuid().ToString('N').Substring(0,8))
    New-Item -ItemType Directory -Path $release -Force | Out-Null
    $source = & (Join-Path $PSScriptRoot 'prepare-publication.ps1') -Destination (Join-Path $release 'staging')
    $offline = & (Join-Path $PSScriptRoot 'prepare-publication.ps1') -Destination (Join-Path $release 'staging') -BundleDependencies
    Copy-Item -LiteralPath $source.Archive -Destination (Join-Path $release "mcdu-doom-$version-source.zip")
    $legalClearance = $source.LegalClearance
    $offlineName = "mcdu-doom-$version-source-offline.zip"
    Copy-Item -LiteralPath $offline.Archive -Destination (Join-Path $release $offlineName)
    $sourceHash = (Get-FileHash -LiteralPath (Join-Path $release $offlineName) -Algorithm SHA256).Hash.ToLowerInvariant()
    & cargo build --release --locked --offline --bins --target x86_64-pc-windows-msvc | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'Windows release build failed.' }
    $portable = Join-Path $release 'staging/portable'
    $symbols = Join-Path $release 'staging/symbols'
    New-Item -ItemType Directory -Path $portable,$symbols -Force | Out-Null
    # Retain the reviewed host sources/docs beside binaries so relative links
    # and the licensing evidence remain usable in the portable directory.
    foreach ($entry in (Get-ChildItem -LiteralPath $source.Directory -Force | Where-Object Name -ne 'PUBLICATION-MANIFEST.json')) {
        Copy-Item -LiteralPath $entry.FullName -Destination $portable -Recurse
    }
    foreach ($binary in @('mcdu-doom','mcdu-doom-tray')) {
        Copy-Item -LiteralPath "target/x86_64-pc-windows-msvc/release/$binary.exe" -Destination $portable
        Copy-Item -LiteralPath ("target/x86_64-pc-windows-msvc/release/" + $binary.Replace('-','_') + '.pdb') -Destination $symbols
    }
    [IO.File]::WriteAllText((Join-Path $portable 'START-HERE.md'), @"
# MCDU DOOM $version — Windows x64 review build

1. Extract this directory to a writable location.
2. In PowerShell 7 run ./scripts/setup-freedoom.ps1 to download pinned game data.
3. Double-click mcdu-doom-tray.exe, or run mcdu-doom.exe --help.
4. Offline check: ./mcdu-doom.exe serve --no-tray --no-usb --seconds 3

The Microsoft Visual C++ v14 x64 runtime (VCRUNTIME140.dll) must be installed;
it is not bundled here. No WAD, firmware or personal configuration is included. USB is used by default
in tray/service mode. Full build sources are in the adjacent $offlineName archive;
its SHA-256 is $sourceHash. Extract it, install Rust/MSVC, and run
cargo build --release --locked --offline --bins --target x86_64-pc-windows-msvc.
Rust, MSVC and Windows SDK are external build prerequisites, not bundled tools.
This package is unsigned. See docs/licensing/review-register.json for the recorded
publication status and LICENSE-POLICY.md for distribution terms.
"@)
    $inventory = Get-Content -LiteralPath (Join-Path $source.Directory 'DEPENDENCIES.json') -Raw | ConvertFrom-Json
    $components = @($inventory | ForEach-Object {
        [ordered]@{ type='library'; name=$_.name; version=$_.version; 'bom-ref'="pkg:cargo/$($_.name)@$($_.version)"; purl="pkg:cargo/$($_.name)@$($_.version)"; licenses=@(@{expression=$_.selected_package_license}); hashes=@(@{alg='SHA-256'; content=$_.registry_checksum}); properties=@(@{name='cargo:declared-license';value=$_.declared_license}) }
    })
    [ordered]@{ bomFormat='CycloneDX'; specVersion='1.6'; version=1; metadata=@{component=@{type='application';name='mcdu-doom';version=$version}}; components=$components } | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $portable 'SBOM.cdx.json') -Encoding utf8
    [ordered]@{
        version=$version; commit=$commit; dirty=($dirty.Count -ne 0); target='x86_64-pc-windows-msvc'
        rustc=@(& rustc -vV); cargo=(& cargo --version); os=[Environment]::OSVersion.VersionString
        profile='release'; external_runtime='Microsoft Visual C++ v14 x64 runtime (VCRUNTIME140.dll); not bundled'; cargo_lock_sha256=(Get-FileHash Cargo.lock -Algorithm SHA256).Hash.ToLowerInvariant()
        source_archive=$offlineName; source_archive_sha256=$sourceHash; legal_clearance=$legalClearance
        vcs_tools_version=$env:VCToolsVersion; windows_sdk_version=$env:WindowsSDKVersion
        build_command='cargo build --release --locked --offline --bins --target x86_64-pc-windows-msvc'
    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $portable 'BUILD-INFO.json') -Encoding utf8
    Copy-Item -LiteralPath (Join-Path $portable 'BUILD-INFO.json') -Destination $release
    foreach ($kind in @('portable','symbols')) {
        $directory = Join-Path $release "staging/$kind"
        Write-ArtifactManifest -Directory $directory -Kind $kind -Version $version -SourceHash $sourceHash -LegalClearance $legalClearance
        New-ReproducibleArchive -Directory $directory -Archive (Join-Path $release "mcdu-doom-$version-windows-x86_64-$kind.zip")
    }
    $artifacts = @(Get-ChildItem -LiteralPath $release -Filter '*.zip' -File | Sort-Object Name | ForEach-Object { [ordered]@{name=$_.Name;sha256=(Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant();bytes=$_.Length} })
    [ordered]@{schema_version=1;version=$version;commit=$commit;dirty=($dirty.Count -ne 0);legal_clearance=$legalClearance;artifacts=$artifacts} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $release 'RELEASE-MANIFEST.json') -Encoding utf8
    Get-ChildItem -LiteralPath $release -File | Where-Object Name -ne SHA256SUMS.txt | Sort-Object Name | ForEach-Object { (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() + '  ' + $_.Name } | Set-Content -LiteralPath (Join-Path $release 'SHA256SUMS.txt') -Encoding utf8
    & python (Join-Path $PSScriptRoot 'verify-release.py') $release | Out-Host
    if ($LASTEXITCODE -ne 0) { throw 'Release verification failed.' }
    [pscustomobject]@{Directory=$release;Version=$version;Source=(Join-Path $release $offlineName);Portable=$portable;LegalClearance=$legalClearance}
} finally { Pop-Location }
