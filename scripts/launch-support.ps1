# SPDX-License-Identifier: GPL-3.0-only
# Shared build and native argument handling for the launch scripts.
function Get-McduExecutable {
    param(
        [Parameter(Mandatory)][string]$Workspace,
        [ValidateSet('debug', 'release')][string]$Configuration = 'release',
        [switch]$Tray,
        [switch]$SkipBuild
    )
    $name = if ($Tray) { 'mcdu-doom-tray' } else { 'mcdu-doom' }
    if (-not $SkipBuild) {
        $buildArguments = @('build', '--locked', '--manifest-path', (Join-Path $Workspace 'Cargo.toml'), '--bin', $name)
        if ($Configuration -eq 'release') { $buildArguments += '--release' }
        & cargo @buildArguments | Out-Host
        if ($LASTEXITCODE -ne 0) { throw 'Build failed.' }
    }
    $suffix = if ([Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT) { '.exe' } else { '' }
    $executable = Join-Path $Workspace "target/$Configuration/$name$suffix"
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        throw "Executable not found: $executable. Run without -SkipBuild."
    }
    return $executable
}

function ConvertTo-McduProcessArgument {
    param([AllowEmptyString()][string]$Value)
    # Start-Process joins ArgumentList into one Windows command line. Preserve
    # embedded quotes and double trailing backslashes before the closing quote.
    if ($Value -ne '' -and $Value -notmatch '[\s"]') { return $Value }
    $escaped = [regex]::Replace($Value, '(\\*)"', '$1$1\"')
    return '"' + [regex]::Replace($escaped, '(\\+)$', '$1$1') + '"'
}
