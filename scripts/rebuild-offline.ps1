# SPDX-License-Identifier: GPL-3.0-only
[CmdletBinding()]
param([Parameter(Mandatory)][string]$Source)
$ErrorActionPreference = 'Stop'
$sourceRoot = [IO.Path]::GetFullPath($Source)
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$buildRoot = Join-Path $workspace ('target/offline-rebuild-' + [guid]::NewGuid().ToString('N'))
if ([IO.Path]::GetExtension($sourceRoot) -eq '.zip') {
    & (Join-Path $PSScriptRoot 'verify-publication.ps1') -Archive $sourceRoot | Out-Host
    $extracted = Join-Path $buildRoot 'extracted-source'
    Expand-Archive -LiteralPath $sourceRoot -DestinationPath $extracted
    $sourceRoot = $extracted
}
if (-not (Test-Path -LiteralPath (Join-Path $sourceRoot '.cargo/config.toml')) -or
    -not (Test-Path -LiteralPath (Join-Path $sourceRoot 'dependencies') -PathType Container)) {
    throw 'Expected a prepared source-with-dependencies directory.'
}
if ([IO.File]::ReadAllText((Join-Path $sourceRoot '.cargo/config.toml')) -notmatch 'directory = "dependencies"') {
    throw 'Offline dependency config must use the local relative dependencies directory.'
}
$names = @('CARGO_HOME','CARGO_TARGET_DIR','CARGO_NET_OFFLINE','CARGO_INCREMENTAL')
$previous = @{}
foreach ($name in $names) { $previous[$name] = [Environment]::GetEnvironmentVariable($name,'Process') }
try {
    $env:CARGO_HOME = Join-Path $buildRoot 'cargo-home'
    $env:CARGO_TARGET_DIR = Join-Path $buildRoot 'target'
    $env:CARGO_NET_OFFLINE = 'true'
    $env:CARGO_INCREMENTAL = '0'
    New-Item -ItemType Directory -Path $env:CARGO_HOME -Force | Out-Null
    Push-Location -LiteralPath $sourceRoot
    try {
        & cargo build --release --locked --offline --bins --target x86_64-pc-windows-msvc
        if ($LASTEXITCODE -ne 0) { throw 'Empty-cache offline build failed.' }
        & (Join-Path $env:CARGO_TARGET_DIR 'x86_64-pc-windows-msvc/release/mcdu-doom.exe') --version
        if ($LASTEXITCODE -ne 0) { throw 'Rebuilt CLI failed.' }
    } finally { Pop-Location }
    Write-Output "Offline rebuild passed: $buildRoot"
} finally {
    foreach ($name in $names) { [Environment]::SetEnvironmentVariable($name,$previous[$name],'Process') }
}
