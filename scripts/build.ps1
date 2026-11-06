param([string]$InnoCompiler, [switch]$SkipBuild, [string]$ReleaseDirectory)
$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$cargoExe = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargoExe)) { $cargoExe = 'cargo' }

# cargo 和 ISCC 把进度写到 stderr；在 Stop 语义下，任何一行进度都会被当成
# 终止错误，所以调用外部程序时临时放宽，只保留退出码判定。
function Invoke-Native {
    param([string]$FilePath, [string[]]$Arguments)
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $FilePath @Arguments } finally { $ErrorActionPreference = $previous }
}

Push-Location $projectRoot
try {
    $appVersion = [regex]::Match((Get-Content -LiteralPath "Cargo.toml" -Raw), '(?m)^version = "([^"]+)"').Groups[1].Value
    if (-not $appVersion) { throw "Cargo version is missing" }
    if (-not $SkipBuild) {
        Invoke-Native $cargoExe @('test', '--locked', '--no-default-features')
        if ($LASTEXITCODE -ne 0) { throw 'Core tests failed' }
        Invoke-Native $cargoExe @('test', '--locked', '--features', 'ui-tests', '--test', 'list_scroll')
        if ($LASTEXITCODE -ne 0) { throw 'List scroll regression tests failed' }
        Invoke-Native $cargoExe @('build', '--locked', '--release')
        if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    }
    if (-not $ReleaseDirectory) { $ReleaseDirectory = Join-Path $projectRoot 'target\release' }
    $releaseExe = Join-Path $ReleaseDirectory 'command-manager.exe'
    if (-not (Test-Path -LiteralPath $releaseExe)) { throw 'Release executable is missing' }
    if ((Get-Item -LiteralPath $releaseExe).VersionInfo.FileVersion -ne $appVersion) { throw 'Executable version does not match Cargo.toml; rebuild before packaging' }
    New-Item -ItemType Directory -Path (Join-Path $projectRoot 'dist') -Force | Out-Null
    & (Join-Path $PSScriptRoot 'notices.ps1')
    Copy-Item -LiteralPath (Join-Path $projectRoot 'assets\THIRD-PARTY-NOTICES.txt') -Destination (Join-Path $projectRoot 'dist\THIRD-PARTY-NOTICES.txt')
    Copy-Item -LiteralPath $releaseExe -Destination (Join-Path $projectRoot 'dist\command-manager.exe')
    Copy-Item -LiteralPath $releaseExe -Destination (Join-Path $projectRoot "dist\command-manager-$appVersion.exe")
    if (-not $InnoCompiler) {
        $candidates = @((Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'), (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'), (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe'))
        $InnoCompiler = $candidates | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    }
    if (-not $InnoCompiler) { throw 'Inno Setup compiler not found; pass -InnoCompiler' }
    Invoke-Native $InnoCompiler @("/DBuildRoot=$ReleaseDirectory", "/DAppVersion=$appVersion", (Join-Path $projectRoot 'packaging\installer.iss'))
    if ($LASTEXITCODE -ne 0) { throw 'Installer build failed' }
    $sourceZip = Join-Path $projectRoot "dist\CommandManager-$appVersion-source.zip"
    $sourcePaths = @('.cargo','src','tests','assets','docs','scripts','packaging','build.rs','Cargo.toml','Cargo.lock','rust-toolchain.toml','README.md','LICENSE') | ForEach-Object { Join-Path $projectRoot $_ }
    Compress-Archive -LiteralPath $sourcePaths -DestinationPath $sourceZip -Force
    $portablePaths = @($releaseExe, (Join-Path $projectRoot 'README.md'), (Join-Path $projectRoot 'LICENSE'), (Join-Path $projectRoot 'dist\THIRD-PARTY-NOTICES.txt'))
    Compress-Archive -LiteralPath $portablePaths -DestinationPath (Join-Path $projectRoot "dist\CommandManager-$appVersion-windows-x64-portable.zip") -Force
    $hashes = Get-ChildItem -LiteralPath (Join-Path $projectRoot 'dist') -File | Where-Object { $_.Name -ne 'SHA256SUMS.txt' } | Get-FileHash -Algorithm SHA256
    $hashes | ForEach-Object { "$($_.Hash.ToLowerInvariant())  $([System.IO.Path]::GetFileName($_.Path))" } | Set-Content -LiteralPath (Join-Path $projectRoot 'dist\SHA256SUMS.txt') -Encoding utf8
    $hashes | Format-Table -AutoSize
} finally { Pop-Location }
