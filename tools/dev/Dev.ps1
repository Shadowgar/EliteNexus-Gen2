# Local commands only. No AI, polling, automatic rebuilds or workspace test defaults.
[CmdletBinding()]
param(
    [Parameter(Mandatory=$true, Position=0)]
    [ValidateSet('Check','Test','Build','Run','Usage')]
    [string]$Action,
    [ValidateSet('nexus-app','nexus-ai','nexus-binds','nexus-data','nexus-eddn','nexus-galaxy','nexus-hud','nexus-journal','nexus-model','nexus-spansh','nexus-update','nexus-uplink')]
    [string]$Package = 'nexus-app',
    [string]$Filter = '',
    [switch]$AllPackageTests,
    [switch]$Lint,
    [switch]$Release,
    [string]$JournalDir = '',
    [string[]]$SessionLog = @(),
    [int]$Jobs = 4
)
$ErrorActionPreference = 'Stop'
if ($Jobs -lt 1 -or $Jobs -gt 32) { throw 'Jobs must be between 1 and 32.' }
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Push-Location $root
try {
    if ($Action -eq 'Usage') {
        if (-not (Get-Command python -ErrorAction SilentlyContinue)) { throw 'Python 3.10+ is required for the local usage report.' }
        & python (Join-Path $PSScriptRoot 'usage.py') @SessionLog
        exit $LASTEXITCODE
    }
    if ($Action -eq 'Test' -and -not $Filter -and -not $AllPackageTests) {
        throw 'Select -Filter for a focused test, or explicitly choose -AllPackageTests.'
    }
    if ($Filter -and $AllPackageTests) { throw 'Choose either a filter or all package tests.' }
    if ($Action -eq 'Build' -and $Package -ne 'nexus-app') { throw 'Build produces the Praetorium application; use Check for another crate.' }
    if ($Action -ne 'Build' -and $Action -ne 'Run' -and $Release) { throw '-Release is supported only by Build and Run.' }
    . (Join-Path $root 'tools\env.ps1')
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) { throw 'Cargo is unavailable.' }
    $target = if ($env:CARGO_TARGET_DIR) { [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR) } else { Join-Path $root 'target' }
    if ($Action -eq 'Run') {
        $profile = if ($Release) { 'release' } else { 'debug' }
        $exe = Join-Path $target "$profile\Praetorium.exe"
        if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { throw "No executable at $exe. Run Build explicitly first." }
        $launchArgs = @('--isolated','--mute')
        if ($JournalDir) {
            $source = (Resolve-Path -LiteralPath $JournalDir).Path
            if (-not (Test-Path -LiteralPath $source -PathType Container)) { throw 'JournalDir must be a directory.' }
            $launchArgs += @('--journal-dir',$source)
        }
        # Do not inherit a normal or previous profile into the isolated run.
        $previousHome = $env:PRAETORIUM_HOME
        try {
            Remove-Item Env:PRAETORIUM_HOME -ErrorAction SilentlyContinue
            & $exe @launchArgs
            exit $LASTEXITCODE
        } finally {
            if ($null -ne $previousHome) { $env:PRAETORIUM_HOME = $previousHome }
            else { Remove-Item Env:PRAETORIUM_HOME -ErrorAction SilentlyContinue }
        }
    }
    $verb = switch ($Action) {
        'Check' { if ($Lint) { 'clippy' } else { 'check' } }
        'Test' { 'test' }
        'Build' { 'build' }
    }
    $cargoArgs = @('+1.100.0',$verb,'--locked','-p',$Package,'-j',"$Jobs")
    if ($Action -eq 'Build' -and $Release) { $cargoArgs += '--release' }
    if ($Action -eq 'Test') {
        if ($Package -eq 'nexus-app') { $cargoArgs += @('--bin','Praetorium') }
        else { $cargoArgs += '--lib' }
        if ($Filter) { $cargoArgs += $Filter }
    }
    if ($Action -eq 'Check' -and $Lint) { $cargoArgs += @('--','-D','warnings') }
    $revision = & git rev-parse --short HEAD
    Write-Host "Revision: $revision; target: $target"
    Write-Host ('cargo ' + ($cargoArgs -join ' '))
    [System.Diagnostics.Process]::GetCurrentProcess().PriorityClass = 'BelowNormal'
    & cargo @cargoArgs
    exit $LASTEXITCODE
} finally {
    Pop-Location
}
