$ErrorActionPreference = 'Stop'
$projectDirectory = Split-Path -Parent $PSScriptRoot
Push-Location $projectDirectory
try {
    cargo build --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    $outputDirectory = Join-Path $projectDirectory 'dist'
    New-Item -ItemType Directory -Force -Path $outputDirectory | Out-Null
    Copy-Item -LiteralPath 'target/release/fantasy-lcsc.exe' -Destination $outputDirectory
    Copy-Item -LiteralPath 'README.md' -Destination $outputDirectory
    Copy-Item -LiteralPath 'LICENSE' -Destination $outputDirectory
    $docsDirectory = Join-Path $outputDirectory 'docs'
    New-Item -ItemType Directory -Force -Path $docsDirectory | Out-Null
    Get-ChildItem -LiteralPath 'docs' -Filter '*.md' -File | Copy-Item -Destination $docsDirectory
    $archiveInputs = @(
        (Join-Path $outputDirectory 'fantasy-lcsc.exe'),
        (Join-Path $outputDirectory 'README.md'),
        (Join-Path $outputDirectory 'LICENSE'),
        $docsDirectory
    )
    Compress-Archive -LiteralPath $archiveInputs -DestinationPath (Join-Path $outputDirectory 'FantasyLCSC-windows-x64.zip') -Force
    Write-Host "Portable CLI: $outputDirectory"
} finally { Pop-Location }
