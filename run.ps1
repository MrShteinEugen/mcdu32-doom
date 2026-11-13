# SPDX-License-Identifier: GPL-3.0-only
param(
    [switch]$Hardware,
    [switch]$Compare,
    [switch]$PreviewOnly,
    [switch]$Headless,
    [switch]$NoTray,
    [switch]$NoUsb,
    [ValidateSet('debug', 'release')]
    [string]$Configuration = 'release',
    [switch]$SkipBuild,
    [ValidateSet('full', 'dense', 'standard')]
    [string]$Display = 'full',
    [ValidateSet('', 'contrast', 'color', 'ascii')]
    [string]$Style = '',
    [ValidateRange(500, 20000)]
    [int]$PacketUs = 10000,
    [ValidateRange(0, 2147483647)]
    [int]$Seconds = 0,
    [string]$Capture = '',
    [string]$Wad = 'data/freedoom-0.13.0/freedoom1.wad'
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'scripts/launch-support.ps1')
if ($Hardware -and $PreviewOnly) { throw 'Choose either Hardware or PreviewOnly.' }
if ($Hardware -and $NoUsb) { throw 'Hardware cannot be combined with NoUsb.' }
if ($Compare -and ($Hardware -or $PreviewOnly)) { throw 'Choose Compare or a hardware/preview mode.' }
$serviceMode = -not $Compare -and -not $PreviewOnly -and -not $Headless
if (($NoTray -or $NoUsb) -and -not $serviceMode) { throw 'NoTray and NoUsb apply to the service; omit Compare, PreviewOnly and Headless.' }
if (-not $serviceMode -and $Style -eq 'ascii') { throw 'Use -Style ascii with the service, optionally with -NoTray -NoUsb.' }
Push-Location -LiteralPath $PSScriptRoot
try {
    $executable = Get-McduExecutable -Workspace $PSScriptRoot -Configuration $Configuration -Tray:($serviceMode -and -not $NoTray) -SkipBuild:$SkipBuild
    if ($serviceMode) {
        $mode = switch ($Style) { 'color' { 'glyphs' } 'ascii' { 'ascii' } 'contrast' { 'contrast' } default { '' } }
        $trayArguments = @('serve', '--wad', $Wad, '--display', $Display, '--packet-us', $PacketUs.ToString())
        if ($mode) { $trayArguments += @('--mode', $mode) }
        if ($Seconds -gt 0) { $trayArguments += @('--seconds', $Seconds.ToString()) }
        if ($Capture) { $trayArguments += @('--capture', $Capture) }
        if ($NoUsb) { $trayArguments += '--no-usb' }
        if ($NoTray) {
            & $executable @trayArguments '--no-tray'
            if ($LASTEXITCODE -ne 0) { throw 'Service stopped with an error.' }
        } else {
            $quotedArguments = @($trayArguments | ForEach-Object { ConvertTo-McduProcessArgument $_ })
            Start-Process -FilePath $executable -ArgumentList $quotedArguments -WorkingDirectory $PSScriptRoot -WindowStyle Hidden
        }
        return
    }
    $renderStyle = if ($Style) { $Style } else { 'contrast' }
    $gameArguments = @('compare', '--wad', $Wad, '--style', $renderStyle)
    if ($Hardware -or $PreviewOnly) {
        $gameArguments[0] = 'play'
        $gameArguments += @('--display', $Display, '--packet-us', $PacketUs.ToString())
    }
    if ($Hardware) { $gameArguments += '--mcdu' }
    if ($Headless) { $gameArguments += '--headless' }
    if ($Seconds -gt 0) { $gameArguments += @('--seconds', $Seconds.ToString()) }
    if ($Capture) { $gameArguments += @('--capture', $Capture) }
    & $executable @gameArguments
    if ($LASTEXITCODE -ne 0) { throw 'Game stopped with an error.' }
} finally {
    Pop-Location
}
