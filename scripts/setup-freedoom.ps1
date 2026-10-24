[CmdletBinding()]
param(
    [string]$DataDirectory = (Join-Path $PSScriptRoot '..\data')
)

$ErrorActionPreference = 'Stop'
$version = '0.13.0'
$archiveSha256 = '3f9b264f3e3ce503b4fb7f6bdcb1f419d93c7b546f4df3e874dd878db9688f59'
$wadSha256 = '7323bcc168c5a45ff10749b339960e98314740a734c30d4b9f3337001f9e703d'
$dataRoot = [System.IO.Path]::GetFullPath($DataDirectory)
$releaseDirectory = Join-Path $dataRoot "freedoom-$version"
$wadPath = Join-Path $releaseDirectory 'freedoom1.wad'

if (Test-Path -LiteralPath $wadPath) {
    if ((Get-FileHash -LiteralPath $wadPath -Algorithm SHA256).Hash -ne $wadSha256) {
        throw 'Existing Freedoom data differs from the pinned release. Choose a separate DataDirectory.'
    }
    Write-Output "Freedoom $version already installed: $wadPath"
    return
}
if (Test-Path -LiteralPath $releaseDirectory) {
    throw "Incomplete installation exists at $releaseDirectory. Choose a separate DataDirectory."
}

$null = New-Item -ItemType Directory -Path $dataRoot -Force
$archivePath = Join-Path $dataRoot ("freedoom-download-" + [guid]::NewGuid().ToString('N') + '.zip')
try {
    $url = "https://github.com/freedoom/freedoom/releases/download/v$version/freedoom-$version.zip"
    Invoke-WebRequest -Uri $url -OutFile $archivePath
    if ((Get-FileHash -LiteralPath $archivePath -Algorithm SHA256).Hash -ne $archiveSha256) {
        throw 'Freedoom archive SHA-256 does not match the official release checksum.'
    }
    Expand-Archive -LiteralPath $archivePath -DestinationPath $dataRoot
    if (-not (Test-Path -LiteralPath $wadPath) -or
        (Get-FileHash -LiteralPath $wadPath -Algorithm SHA256).Hash -ne $wadSha256) {
        throw 'Extracted Freedoom WAD failed validation.'
    }
    Write-Output "Freedoom $version installed: $wadPath"
} finally {
    if (Test-Path -LiteralPath $archivePath) { Remove-Item -LiteralPath $archivePath -Force }
}
