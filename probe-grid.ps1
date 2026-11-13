# SPDX-License-Identifier: GPL-3.0-only
param(
    [Alias('Profile')]
    [ValidateSet('Baseline', 'Compact', 'Wide', 'Tall', 'Dense', 'DenseTight', 'DenseCaptured')]
    [string]$ProbeProfile = 'Compact',
    [ValidateSet('captured', 'standalone', 'begin')]
    [string]$Layout = 'captured',
    [ValidateSet('before', 'after', 'both')]
    [string]$GridOrder = 'after',
    [ValidateSet('tight', 'padded', 'captured')]
    [string]$Packing = 'tight',
    [ValidateSet('coordinates', 'solid', 'checkerboard')]
    [string]$Pattern = 'coordinates',
    [switch]$Hardware,
    [switch]$KeepFont,
    [string]$DevicePath = '',
    [ValidateSet('debug', 'release')]
    [string]$Configuration = 'release',
    [switch]$SkipBuild
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'scripts/launch-support.ps1')
if ($DevicePath -and -not $Hardware) { throw 'DevicePath requires Hardware.' }
Push-Location -LiteralPath $PSScriptRoot
try {
    $executable = Get-McduExecutable -Workspace $PSScriptRoot -Configuration $Configuration -SkipBuild:$SkipBuild
    if ($ProbeProfile -eq 'DenseTight') { $Packing = 'tight' }
    if ($ProbeProfile -eq 'DenseCaptured') { $Packing = 'captured' }
    $outputName = "$ProbeProfile-$Packing-$Layout-$GridOrder-$Pattern"
    if ($KeepFont) { $outputName += '-keep-font' }
    $probeArguments = @('probe', '--layout', $Layout, '--grid-order', $GridOrder,
        '--packing', $Packing, '--pattern', $Pattern,
        '--out', "output/grid-probe/$outputName")
    switch ($ProbeProfile) {
        'Baseline' { $probeArguments += @('--columns', '24', '--rows', '14', '--glyph-width', '23', '--glyph-height', '29', '--baseline') }
        'Compact'  { $probeArguments += @('--columns', '24', '--rows', '14') }
        'Wide'     { $probeArguments += @('--columns', '48', '--rows', '14') }
        'Tall'     { $probeArguments += @('--columns', '24', '--rows', '28') }
        'Dense'    { $probeArguments += @('--columns', '48', '--rows', '28') }
        'DenseTight' { }
        'DenseCaptured' { }
    }
    if ($Hardware) { $probeArguments += '--mcdu' }
    if ($KeepFont) { $probeArguments += '--skip-font' }
    if ($DevicePath) {
        $probeArguments += @('--device-path', $DevicePath)
    }
    & $executable @probeArguments
    if ($LASTEXITCODE -ne 0) { throw 'Display probe failed; inspect its manifest.json.' }
} finally {
    Pop-Location
}
