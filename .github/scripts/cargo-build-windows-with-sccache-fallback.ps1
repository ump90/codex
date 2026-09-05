[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet("x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc")]
    [string]$Target,

    [Parameter(Mandatory)]
    [string[]]$Binaries
)

$ErrorActionPreference = "Stop"

$buildArgs = @("build", "--target", $Target, "--release", "--timings")
foreach ($binary in $Binaries) {
    $buildArgs += @("--bin", $binary)
}

$logPath = Join-Path -Path $env:RUNNER_TEMP -ChildPath "cargo-build-$Target.log"
& cargo @buildArgs 2>&1 | Tee-Object -FilePath $logPath
$initialExitCode = $LASTEXITCODE

if ($initialExitCode -eq 0) {
    return
}

if (-not (Select-String -LiteralPath $logPath -Pattern "sccache: caused by:" -SimpleMatch -Quiet)) {
    throw "Cargo build failed with exit code $initialExitCode."
}

Write-Warning "sccache lost its connection to the GitHub Actions cache service; retrying without sccache."
Remove-Item Env:RUSTC_WRAPPER -ErrorAction SilentlyContinue
Remove-Item Env:SCCACHE_GHA_ENABLED -ErrorAction SilentlyContinue

& cargo @buildArgs
if ($LASTEXITCODE -ne 0) {
    throw "Cargo retry without sccache failed with exit code $LASTEXITCODE."
}
