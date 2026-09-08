[CmdletBinding()]
param(
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

$installRoot = Join-Path $env:LOCALAPPDATA "FahdCodexRouter\bin"
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$pathEntries = @($userPath -split ";" | Where-Object { $_ -and $_ -ne $installRoot })

if ($Uninstall) {
    if (Test-Path -LiteralPath $installRoot) {
        Remove-Item -LiteralPath $installRoot -Recurse -Force
    }
    [Environment]::SetEnvironmentVariable("Path", ($pathEntries -join ";"), "User")
    Write-Host "Removed the personal Codex router. Open a new terminal to use the official CLI."
    exit 0
}

$requiredFiles = @(
    "codex.exe"
    "codex-code-mode-host.exe"
    "codex-command-runner.exe"
    "codex-windows-sandbox-setup.exe"
)

foreach ($file in $requiredFiles) {
    $source = Join-Path $PSScriptRoot $file
    if (-not (Test-Path -LiteralPath $source)) {
        throw "Missing $file next to this installer. Extract the complete ZIP before running it."
    }
}

New-Item -ItemType Directory -Force -Path $installRoot | Out-Null
foreach ($file in $requiredFiles) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $file) -Destination $installRoot -Force
}

$officialLauncher = Join-Path $env:APPDATA "npm\codex.cmd"
if (Test-Path -LiteralPath $officialLauncher) {
    $fallback = Join-Path $installRoot "codex-official.cmd"
    '@"%APPDATA%\npm\codex.cmd" %*' | Set-Content -LiteralPath $fallback -Encoding ascii
}

$newUserPath = (@($installRoot) + $pathEntries) -join ";"
[Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")

Write-Host "Installed the personal Codex router at $installRoot"
Write-Host "Open a new terminal, then run: codex"
if (Test-Path -LiteralPath $officialLauncher) {
    Write-Host "To run OpenAI's installed CLI instead: codex-official"
}
