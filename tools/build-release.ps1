# Builds and packages a local Praetorium development copy, without publication or update signing.
#   powershell -ExecutionPolicy Bypass -File .\tools\build-release.ps1
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'env.ps1')
Set-Location $root

$name = 'Praetorium'
$exe = "$name.exe"
$dist = Join-Path $root "dist\$name"
$target = Join-Path $dist $exe
$running = Get-Process -Name $name -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $target }
if ($running) { throw "Close $name before replacing this local copy." }

cargo +1.100.0 build --locked --release -p nexus-app
if ($LASTEXITCODE -ne 0) { throw 'cargo build failed' }

$version = (Select-String -LiteralPath (Join-Path $root 'Cargo.toml') -Pattern '^version = "(.+)"' | Select-Object -First 1).Matches[0].Groups[1].Value
$targetDir = if ($env:CARGO_TARGET_DIR) { [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR) } else { Join-Path $root 'target' }
$built = Join-Path $targetDir "release\$exe"
$docSources = @('LICENSE', 'README.md', 'NOTICE.md', 'docs\USER_GUIDE.md', 'docs\LICENSING.md', 'docs\UPSTREAM-ACKNOWLEDGMENTS.md', 'docs\PROFILE_MIGRATION.md', 'docs\THIRD-PARTY-LICENSES.txt')
$docFiles = @($docSources | ForEach-Object { Split-Path -Leaf $_ }) + @('Inter-LICENSE.txt')
New-Item -ItemType Directory -Force -Path $dist | Out-Null
foreach ($f in $docSources) { Copy-Item -LiteralPath (Join-Path $root $f) -Destination $dist -Force }
Copy-Item -LiteralPath (Join-Path $root 'data\fonts\inter\LICENSE.txt') -Destination (Join-Path $dist 'Inter-LICENSE.txt') -Force
Copy-Item -LiteralPath $built -Destination $target -Force

# Package only the executable and named documents. Both application profiles are outside the archive.
$zip = Join-Path $root "dist\$name-v$version-windows-x64.zip"
$files = @($target) + @($docFiles | ForEach-Object { Join-Path $dist $_ })
Compress-Archive -LiteralPath $files -DestinationPath $zip -Force
"Built local $name $version -> $zip"
