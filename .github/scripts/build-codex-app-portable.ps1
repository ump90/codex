param(
    [Parameter(Mandatory = $true)]
    [string]$Target,

    [Parameter(Mandatory = $true)]
    [string]$CodexPackageDir,

    [Parameter(Mandatory = $true)]
    [string]$DistRoot,

    [Parameter(Mandatory = $true)]
    [string]$SourceUrl,

    [Parameter(Mandatory = $true)]
    [string]$ExpectedSha256Base64,

    [Parameter(Mandatory = $true)]
    [string]$BaseAppVersion,

    [Parameter(Mandatory = $true)]
    [string]$BasePackageVersion,

    [Parameter(Mandatory = $true)]
    [string]$ForkReleaseTag,

    [Parameter(Mandatory = $true)]
    [string]$SourceRef
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-SevenZip {
    $command = Get-Command 7z -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }

    $defaultPath = Join-Path -Path ${env:ProgramFiles} -ChildPath "7-Zip\7z.exe"
    if (Test-Path -LiteralPath $defaultPath) {
        return $defaultPath
    }

    throw "7-Zip was not found on PATH or at $defaultPath"
}

function Reset-ChildDirectory {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,

        [Parameter(Mandatory = $true)]
        [string]$Root
    )

    $fullPath = [IO.Path]::GetFullPath($Path).TrimEnd([IO.Path]::DirectorySeparatorChar)
    $fullRoot = [IO.Path]::GetFullPath($Root).TrimEnd([IO.Path]::DirectorySeparatorChar)
    $rootPrefix = "$fullRoot$([IO.Path]::DirectorySeparatorChar)"
    if (-not $fullPath.StartsWith($rootPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to reset directory outside $fullRoot`: $fullPath"
    }

    if (Test-Path -LiteralPath $fullPath) {
        Remove-Item -LiteralPath $fullPath -Recurse -Force
    }
    New-Item -ItemType Directory -Path $fullPath -Force | Out-Null
}

$appArchitecture = switch ($Target) {
    "x86_64-pc-windows-msvc" { "x64"; break }
    "aarch64-pc-windows-msvc" { "arm64"; break }
    default { throw "Unsupported Codex App target: $Target" }
}

$sevenZip = Get-SevenZip
$downloadDir = Join-Path -Path $env:RUNNER_TEMP -ChildPath "codex-app-download-$Target"
$extractDir = Join-Path -Path $env:RUNNER_TEMP -ChildPath "codex-app-extract-$Target"
$packageDir = Join-Path -Path $DistRoot -ChildPath "codex-app-portable-$Target"
$archivePath = Join-Path -Path $DistRoot -ChildPath "codex-app-portable-windows-$Target.zip"
$msixPath = Join-Path -Path $downloadDir -ChildPath "Codex-Windows-$appArchitecture.msix"

Reset-ChildDirectory -Path $downloadDir -Root $env:RUNNER_TEMP
Reset-ChildDirectory -Path $extractDir -Root $env:RUNNER_TEMP
Reset-ChildDirectory -Path $packageDir -Root $DistRoot

Write-Host "Downloading Codex App $BaseAppVersion for $appArchitecture"
& curl.exe --fail --location --retry 3 --retry-all-errors --output $msixPath $SourceUrl
if ($LASTEXITCODE -ne 0) {
    throw "Failed to download Codex App from $SourceUrl"
}

$expectedSha256 = [Convert]::ToHexString(
    [Convert]::FromBase64String($ExpectedSha256Base64)
).ToLowerInvariant()
$actualSha256 = (Get-FileHash -LiteralPath $msixPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualSha256 -ne $expectedSha256) {
    throw "Codex App MSIX SHA-256 mismatch. Expected $expectedSha256, got $actualSha256"
}

& $sevenZip x $msixPath "-o$extractDir" -y
if ($LASTEXITCODE -ne 0) {
    throw "Failed to extract $msixPath"
}

$manifestPath = Join-Path -Path $extractDir -ChildPath "AppxManifest.xml"
if (-not (Test-Path -LiteralPath $manifestPath)) {
    throw "Extracted Codex App package is missing AppxManifest.xml"
}

[xml]$appxManifest = Get-Content -LiteralPath $manifestPath -Raw
$identity = $appxManifest.SelectSingleNode("/*[local-name()='Package']/*[local-name()='Identity']")
if (-not $identity) {
    throw "Extracted Codex App package manifest is missing Package/Identity"
}
if ($identity.ProcessorArchitecture -ne $appArchitecture) {
    throw "Codex App architecture mismatch. Expected $appArchitecture, got $($identity.ProcessorArchitecture)"
}
if ($identity.Version -ne $BasePackageVersion) {
    throw "Codex App package version mismatch. Expected $BasePackageVersion, got $($identity.Version)"
}

$appSourceDir = Join-Path -Path $extractDir -ChildPath "app"
$appDir = Join-Path -Path $packageDir -ChildPath "app"
if (-not (Test-Path -LiteralPath (Join-Path -Path $appSourceDir -ChildPath "ChatGPT.exe"))) {
    throw "Extracted Codex App package is missing app\ChatGPT.exe"
}
Move-Item -LiteralPath $appSourceDir -Destination $appDir
Remove-Item -LiteralPath $msixPath -Force
Remove-Item -LiteralPath $extractDir -Recurse -Force

$resourcesDir = Join-Path -Path $appDir -ChildPath "resources"
$sidecarSources = [ordered]@{
    "codex.exe" = "bin\codex.exe"
    "codex-code-mode-host.exe" = "bin\codex-code-mode-host.exe"
    "codex-command-runner.exe" = "codex-resources\codex-command-runner.exe"
    "codex-windows-sandbox-setup.exe" = "codex-resources\codex-windows-sandbox-setup.exe"
}
foreach ($sidecar in $sidecarSources.Keys) {
    $source = Join-Path -Path $CodexPackageDir -ChildPath $sidecarSources[$sidecar]
    $destination = Join-Path -Path $resourcesDir -ChildPath $sidecar
    if (-not (Test-Path -LiteralPath $source)) {
        throw "Compiled Codex sidecar is missing: $source"
    }
    if (-not (Test-Path -LiteralPath $destination)) {
        throw "Codex App does not contain the expected sidecar: $destination"
    }
    Copy-Item -LiteralPath $source -Destination $destination -Force
}

$portableGitSource = Join-Path -Path $CodexPackageDir -ChildPath "git"
$portableGitDestination = Join-Path -Path $resourcesDir -ChildPath "git"
foreach ($relativePath in @("cmd\git.exe", "bin\bash.exe", "usr\bin\msys-2.0.dll")) {
    if (-not (Test-Path -LiteralPath (Join-Path -Path $portableGitSource -ChildPath $relativePath))) {
        throw "Portable Git source is missing $relativePath"
    }
}
Copy-Item -LiteralPath $portableGitSource -Destination $portableGitDestination -Recurse -Force

@'
@echo off
set "CODEX_APP_PORTABLE_ROOT=%~dp0"
set "PATH=%CODEX_APP_PORTABLE_ROOT%app\resources\git\cmd;%CODEX_APP_PORTABLE_ROOT%app\resources\git\bin;%PATH%"
start "" "%CODEX_APP_PORTABLE_ROOT%app\ChatGPT.exe" %*
exit /b %ERRORLEVEL%
'@ | Set-Content -LiteralPath (Join-Path -Path $packageDir -ChildPath "codex-app.cmd") -Encoding ascii

@"
Portable Codex App for Windows ($appArchitecture)

Run codex-app.cmd to launch the app. The launcher prepends the bundled Git for
Windows directories to PATH so the custom Codex binary can discover Git Bash.

This archive is derived from the official Codex App MSIX, but it is not an
installable MSIX. Replacing the Codex sidecars invalidates the original package
signature, so package metadata and signatures are intentionally not included.
Use a newly published archive to update this portable installation.

Base Codex App version: $BaseAppVersion
Base Windows package version: $BasePackageVersion
Fork release: $ForkReleaseTag
Source ref: $SourceRef
Original MSIX SHA-256: $actualSha256
"@ | Set-Content -LiteralPath (Join-Path -Path $packageDir -ChildPath "PORTABLE-INSTALL.txt") -Encoding utf8

$buildInfo = [ordered]@{
    schemaVersion = 1
    architecture = $appArchitecture
    baseCodexAppVersion = $BaseAppVersion
    baseWindowsPackageVersion = $BasePackageVersion
    forkReleaseTag = $ForkReleaseTag
    sourceRef = $SourceRef
    sourceUrl = $SourceUrl
    originalMsixSha256 = $actualSha256
    replacedSidecars = @($sidecarSources.Keys)
}
$buildInfo | ConvertTo-Json -Depth 3 | Set-Content `
    -LiteralPath (Join-Path -Path $packageDir -ChildPath "CODEX-APP-BUILD-INFO.json") `
    -Encoding utf8

if (Test-Path -LiteralPath $archivePath) {
    Remove-Item -LiteralPath $archivePath -Force
}
Push-Location $packageDir
try {
    & $sevenZip a -tzip $archivePath ".\*"
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to create $archivePath"
    }
} finally {
    Pop-Location
}

$packageFiles = @(Get-ChildItem -Path $packageDir -Recurse -File)
$packageSizeMiB = [math]::Round((($packageFiles | Measure-Object -Property Length -Sum).Sum) / 1MB, 1)
Write-Host "Portable Codex App files: $($packageFiles.Count); unpacked size: $packageSizeMiB MiB"
Get-Item -LiteralPath $archivePath | Select-Object FullName, Length
