$ErrorActionPreference = 'Stop'
$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$cargoExe = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargoExe)) { $cargoExe = 'cargo' }
Push-Location $projectRoot
$previousEncoding = [Console]::OutputEncoding
# cargo writes UTF-8; the console code page would otherwise mangle the paths and
# produce JSON that ConvertFrom-Json rejects.
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
try {
    $metadataText = & $cargoExe metadata --locked --offline --format-version 1 --filter-platform x86_64-pc-windows-msvc
    if ($LASTEXITCODE -ne 0) { throw 'Cannot enumerate locked dependencies' }
    $metadata = ($metadataText -join "`n") | ConvertFrom-Json
    $output = [System.Text.StringBuilder]::new()
    [void]$output.AppendLine('Command Manager - Third-party notices')
    [void]$output.AppendLine('Application: MIT (see LICENSE).')
    [void]$output.AppendLine('Dependency versions and declared licenses below come from Cargo.lock metadata.')
    [void]$output.AppendLine("`nEmbedded Geist font:")
    [void]$output.AppendLine([System.IO.File]::ReadAllText((Join-Path $projectRoot 'assets\LICENSE-GEIST')))
    $packages = $metadata.packages | Sort-Object name, version
    foreach ($package in $packages) {
        [void]$output.AppendLine("`n============================================================")
        [void]$output.AppendLine("$($package.name) $($package.version)")
        [void]$output.AppendLine("License: $($package.license)")
        [void]$output.AppendLine("Repository: $($package.repository)")
        $packageRoot = Split-Path -Parent $package.manifest_path
        $licenseFiles = @(Get-ChildItem -LiteralPath $packageRoot -File | Where-Object { $_.Name -match '^(LICENSE|LICENCE|COPYING|UNLICENSE)' })
        if ($package.license_file) {
            $licensePath = Join-Path $packageRoot $package.license_file
            if (Test-Path -LiteralPath $licensePath -PathType Leaf) { $licenseFiles += Get-Item -LiteralPath $licensePath }
        }
        foreach ($file in ($licenseFiles | Sort-Object FullName -Unique)) {
            [void]$output.AppendLine("--- $($file.Name) ---")
            [void]$output.AppendLine([System.IO.File]::ReadAllText($file.FullName))
        }
    }
    $noticesPath = Join-Path $projectRoot 'assets\THIRD-PARTY-NOTICES.txt'
    [System.IO.File]::WriteAllText($noticesPath, $output.ToString(), [System.Text.UTF8Encoding]::new($false))
    Write-Output "Notices generated for $($packages.Count) locked packages."
} finally { Pop-Location; [Console]::OutputEncoding = $previousEncoding }
