param(
    [string]$Msys2Root = 'C:\msys64',
    [switch]$Installer,
    [string]$InnoSetup = 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe'
)

$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot
$ucrt = Join-Path $Msys2Root 'ucrt64'
$rustBin = Join-Path $env:USERPROFILE ".cargo/bin"
$env:PATH = "$(Join-Path $ucrt 'bin');$rustBin;$env:PATH"
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'

function Invoke-Checked {
    param([string]$Tool, [string[]]$Arguments)
    & $Tool @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Tool failed with exit code $LASTEXITCODE." }
}

Push-Location $projectRoot
try {
    & (Join-Path $PSScriptRoot 'prepare-engine.ps1')
    $toolchains = & rustup toolchain list
    if (-not ($toolchains -match '^stable-x86_64-pc-windows-gnu')) {
        Invoke-Checked 'rustup' @('toolchain', 'install', 'stable-x86_64-pc-windows-gnu', '--profile', 'minimal')
    }
    Invoke-Checked 'cargo' @('build', '--release', '--locked', '--target', 'x86_64-pc-windows-gnu')
    Invoke-Checked 'cmake' @('-S', 'engine/uxplay', '-B', 'engine/uxplay/build', '-G', 'Ninja', '-DCMAKE_BUILD_TYPE=Release', '-DNO_MARCH_NATIVE=ON')
    Invoke-Checked 'cmake' @('--build', 'engine/uxplay/build', '--parallel')

    $runtime = Join-Path $projectRoot 'package/runtime'
    $licenses = Join-Path $projectRoot 'package/licenses'
    New-Item -ItemType Directory -Path $runtime, (Join-Path $runtime 'plugins'), $licenses -Force | Out-Null
    Copy-Item 'target/x86_64-pc-windows-gnu/release/airplay-windows.exe' 'package/airplay-windows.exe' -Force
    Copy-Item 'engine/uxplay/build/uxplay.exe' (Join-Path $runtime 'uxplay.exe') -Force
    Copy-Item 'engine/uxplay/LICENSE' (Join-Path $licenses 'UXPLAY-LICENSE.txt') -Force
    Copy-Item 'LICENSE' (Join-Path $licenses 'Aireluma-LICENSE.txt') -Force

    foreach ($entry in Get-Content 'engine/runtime-files.txt') {
        if (-not $entry) { continue }
        if ($entry.StartsWith('plugins/')) {
            $source = Join-Path $ucrt ('lib/gstreamer-1.0/' + (Split-Path $entry -Leaf))
        } elseif ($entry -eq 'gst-plugin-scanner.exe') {
            $source = Join-Path $ucrt 'libexec/gstreamer-1.0/gst-plugin-scanner.exe'
        } else {
            $source = Join-Path $ucrt ('bin/' + $entry)
        }
        if (-not (Test-Path -LiteralPath $source)) { throw "Missing runtime file: $entry. See docs/BUILDING.md." }
        Copy-Item -LiteralPath $source -Destination (Join-Path $runtime $entry) -Force
    }

    $runtimePath = [IO.Path]::GetFullPath($runtime).TrimEnd('\') + '\'
    $allowedFiles = @('uxplay.exe') + @(Get-Content 'engine/runtime-files.txt' | ForEach-Object { $_.Replace('/', '\') })
    foreach ($file in Get-ChildItem -LiteralPath $runtime -File -Recurse) {
        if (-not $file.FullName.StartsWith($runtimePath, [StringComparison]::OrdinalIgnoreCase)) { throw 'Runtime file is outside the package.' }
        $relativePath = $file.FullName.Substring($runtimePath.Length)
        if ($relativePath -notin $allowedFiles) { Remove-Item -LiteralPath $file.FullName -Force }
    }

    foreach ($package in (Get-Content 'engine/runtime-packages.json' -Raw | ConvertFrom-Json)) {
        $name = $package.package -replace '^mingw-w64-ucrt-x86_64-', ''
        $source = Join-Path $ucrt "share/licenses/$name"
        if (Test-Path $source) {
            $destination = Join-Path $licenses "runtime/$name"
            New-Item -ItemType Directory -Path $destination -Force | Out-Null
            Copy-Item (Join-Path $source '*') $destination -Recurse -Force
        }
    }
    Copy-Item 'engine/runtime-packages.json' $licenses -Force
    if ($Installer) { Invoke-Checked $InnoSetup @('installer.iss') }
} finally {
    Pop-Location
}
