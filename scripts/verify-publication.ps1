# SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Archive)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead([IO.Path]::GetFullPath($Archive))
try {
    $entries = @{}
    foreach ($entry in $zip.Entries) {
        $name = $entry.FullName
        if ($name.Contains('\') -or $name.StartsWith('/') -or $name.Contains(':') -or $name -match '(^|/)\.\.?(/|$)' -or
            $name -match '^(\.git|data|output|research|target|dist)(/|$)' -or
            (-not $name.StartsWith('dependencies/') -and (
                [IO.Path]::GetExtension($name) -in @('.wad','.exe','.dll','.pdb','.dsg','.cfg','.zip','.bin') -or
                ([IO.Path]::GetFileName($name) -like '.env*' -and [IO.Path]::GetFileName($name) -ne '.env.example')))) { throw "Excluded/unsafe publication entry: $name" }
        if ($name.EndsWith('/')) { continue }
        if ($entries.ContainsKey($name)) { throw 'Duplicate or case-colliding archive entries.' }
        $entries[$name] = $entry
    }
    function Read-ArchiveJson {
        param([string]$Name)
        if (-not $entries.ContainsKey($Name)) { throw "Missing archive record: $Name" }
        $reader = [IO.StreamReader]::new($entries[$Name].Open())
        try { $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
    }
    $manifest = Read-ArchiveJson 'PUBLICATION-MANIFEST.json'
    if ($manifest.kind -notin @('source-only','source-with-dependencies') -or $manifest.legal_clearance -ne $false) { throw 'Unsupported or misleading publication status.' }
    if ($manifest.files.Count + 1 -ne $entries.Count) { throw 'Archive/manifest file count differs.' }
    $seen = @{}
    foreach ($file in $manifest.files) {
        if ($seen.ContainsKey($file.path) -or -not $entries.ContainsKey($file.path) -or $file.path -eq 'PUBLICATION-MANIFEST.json') { throw 'Duplicate/missing manifest file.' }
        $seen[$file.path] = $true
        $stream = $entries[$file.path].Open()
        try { $hash = (Get-FileHash -InputStream $stream -Algorithm SHA256).Hash.ToLowerInvariant() } finally { $stream.Dispose() }
        if ($hash -cne $file.sha256) { throw "Archive hash mismatch: $($file.path)" }
    }
    if ($manifest.kind -eq 'source-with-dependencies' -and (-not $entries.ContainsKey('.cargo/config.toml') -or -not @($entries.Keys | Where-Object { $_ -like 'dependencies/*/.cargo-checksum.json' }).Count)) { throw 'Missing offline dependency sources/configuration.' }
    $required = @('LICENSE','LICENSE-POLICY.md','MODIFICATIONS.md','NOTICE.md','THIRD_PARTY.md','CONTRIBUTING.md','README.md','README.ru.md','.github/workflows/ci.yml',
        'docs/licensing/review-register.json','docs/licensing/dependencies-windows.json','docs/licensing/native-components.json','docs/licensing/license-texts.json',
        'assets/aircraft-font/HoneywellMCDU.sfd','assets/aircraft-font/HoneywellMCDUSmall.sfd','assets/aircraft-font/LICENSE','scripts/build-aircraft-font.py','docs/images/LICENSE.md')
    foreach ($name in $required) { if (-not $entries.ContainsKey($name)) { throw "Missing required publication file: $name" } }
    $review = Read-ArchiveJson 'docs/licensing/review-register.json'
    if ($review.legal_clearance -ne $false -or $manifest.publication_status -ne $review.publication_status -or
        (($manifest.open_rights_items | Sort-Object) -join ',') -cne (($review.items | Where-Object status -eq unresolved | ForEach-Object id | Sort-Object) -join ',')) { throw 'Archive review status differs from manifest.' }
    $dependencies = @(Read-ArchiveJson 'DEPENDENCIES.json')
    $reviewed = Read-ArchiveJson 'docs/licensing/dependencies-windows.json'
    if ($manifest.dependency_count -ne $dependencies.Count -or $dependencies.Count -ne $reviewed.packages.Count -or $manifest.target -ne $reviewed.target) { throw 'Dependency inventory/target count differs.' }
    # The checked-in review is sorted; package inventory is created in the same order.
    if (($dependencies | ConvertTo-Json -Depth 30 -Compress) -cne ($reviewed.packages | ConvertTo-Json -Depth 30 -Compress)) { throw 'Published dependency inventory differs from reviewed records.' }
    if ($manifest.kind -eq 'source-with-dependencies') {
        foreach ($name in @($entries.Keys | Where-Object { $_ -like 'dependencies/*/.cargo-checksum.json' })) {
            $checksums = Read-ArchiveJson $name
            $prefix = $name.Substring(0, $name.Length - '.cargo-checksum.json'.Length)
            foreach ($property in $checksums.files.PSObject.Properties) {
                $fileName = $prefix + $property.Name
                if (-not $entries.ContainsKey($fileName)) { throw "Missing vendored file: $fileName" }
                $stream = $entries[$fileName].Open()
                try { $hash = (Get-FileHash -InputStream $stream -Algorithm SHA256).Hash.ToLowerInvariant() } finally { $stream.Dispose() }
                if ($hash -cne $property.Value) { throw "Cargo vendor file checksum mismatch: $fileName" }
            }
        }
    }
    $noticeCount = 0
    foreach ($dependency in $dependencies) {
        foreach ($notice in $dependency.notices) {
            $noticeName = 'THIRD_PARTY_LICENSES/' + $dependency.name + '-' + $dependency.version + '/' + $notice.path
            if (-not $entries.ContainsKey($noticeName)) { throw "Missing nested dependency notice: $noticeName" }
            $stream = $entries[$noticeName].Open()
            try { $hash = (Get-FileHash -InputStream $stream -Algorithm SHA256).Hash.ToLowerInvariant() } finally { $stream.Dispose() }
            if ($hash -cne $notice.sha256) { throw 'Copied dependency notice hash differs.' }
            $noticeCount++
        }
    }
    [pscustomobject]@{ ArchiveFiles=$entries.Count; VerifiedHashes=$manifest.files.Count; DependencyNotices=$noticeCount; LegalClearance=$false; TechnicalChecks='passed' }
} finally { $zip.Dispose() }
