# SPDX-License-Identifier: GPL-3.0-only
Set-StrictMode -Version Latest

function Get-PackageFiles {
    param([string]$Directory)
    @(Get-ChildItem -LiteralPath $Directory -Recurse -File -Force | Sort-Object FullName | ForEach-Object {
        if ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Package files must not be links.' }
        [ordered]@{
            path = [IO.Path]::GetRelativePath($Directory, $_.FullName).Replace('\','/')
            sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
            bytes = $_.Length
        }
    })
}

function New-ReproducibleArchive {
    param([string]$Directory, [string]$Archive)
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    if (Test-Path -LiteralPath $Archive) { throw "Archive already exists: $Archive" }
    $files = @(Get-ChildItem -LiteralPath $Directory -Recurse -File -Force)
    $names = [string[]]@($files | ForEach-Object { [IO.Path]::GetRelativePath($Directory,$_.FullName).Replace('\','/') })
    [Array]::Sort($names, [StringComparer]::Ordinal)
    $zip = [IO.Compression.ZipFile]::Open($Archive, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in $names) {
            $entry = $zip.CreateEntry($name, [IO.Compression.CompressionLevel]::Optimal)
            $entry.LastWriteTime = [DateTimeOffset]::new(2000,1,1,0,0,0,[TimeSpan]::Zero)
            $sourceStream = [IO.File]::OpenRead((Join-Path $Directory $name))
            $destinationStream = $entry.Open()
            try { $sourceStream.CopyTo($destinationStream) } finally { $destinationStream.Dispose(); $sourceStream.Dispose() }
        }
    } finally { $zip.Dispose() }
}

function Write-ArtifactManifest {
    param([string]$Directory, [string]$Kind, [string]$Version, [string]$SourceHash)
    [ordered]@{
        schema_version = 1; kind = $Kind; version = $Version; target = 'x86_64-pc-windows-msvc'
        source_archive_sha256 = $SourceHash; legal_clearance = $false
        files = @(Get-PackageFiles -Directory $Directory)
    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $Directory 'ARTIFACT-MANIFEST.json') -Encoding utf8
}
