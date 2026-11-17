# SPDX-License-Identifier: GPL-3.0-only
# Dot-source in PowerShell to complete commands and flags from the CLI help.
Register-ArgumentCompleter -Native -CommandName mcdu-doom, mcdu-doom.exe, mcdu-doom-tray, mcdu-doom-tray.exe -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $elements = @($commandAst.CommandElements | ForEach-Object { $_.Extent.Text.Trim('"', "'") })
    $executable = Get-Command $elements[0] -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if (-not $executable) { return }
    $helpArguments = @('--help')
    $subcommand = $null
    if ($elements.Count -gt 1 -and $elements[1] -notmatch '^-' -and $elements[1] -ne $wordToComplete) {
        $subcommand = $elements[1]
        $helpArguments = @($subcommand, '--help')
    }
    $helpLines = & $executable.Source @helpArguments 2>$null
    $pattern = if ($subcommand -or $wordToComplete -like '-*') {
        '^\s+(?:-[a-zA-Z],\s+)?(?<name>--[a-z][a-z0-9-]*)'
    } else {
        '^\s{2}(?<name>[a-z][a-z0-9-]*)\s{2,}'
    }
    $helpLines | ForEach-Object {
        if ($_ -match $pattern -and $Matches.name.StartsWith($wordToComplete, [StringComparison]::OrdinalIgnoreCase)) {
            [System.Management.Automation.CompletionResult]::new($Matches.name, $Matches.name, 'ParameterName', $Matches.name)
        }
    }
}
