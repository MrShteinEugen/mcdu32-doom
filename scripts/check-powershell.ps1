# SPDX-License-Identifier: GPL-3.0-only
$ErrorActionPreference = 'Stop'
$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$scripts = @(Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.ps1' -File) +
    @(Get-ChildItem -LiteralPath $workspace -Filter '*.ps1' -File)
foreach ($script in $scripts) {
    $tokens = $null; $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile($script.FullName,[ref]$tokens,[ref]$errors) | Out-Null
    if ($errors.Count) { throw ($script.Name + ': ' + (($errors | ForEach-Object Message) -join '; ')) }
}
Write-Output "Parsed $($scripts.Count) PowerShell scripts."
