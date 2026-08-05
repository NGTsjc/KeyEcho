$ErrorActionPreference = "Stop"
$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..\..")
$distRoot = Join-Path $repoRoot "TempFile\dist\windows"
$assetDir = Join-Path $distRoot "asset"
$binaryPath = Join-Path $repoRoot "TempFile\target\release\KeyEcho.exe"
$configTemplate = Join-Path $PSScriptRoot "config.template.json"
$logoPath = Join-Path $repoRoot "asset\logo.ico"
Write-Host "Building release binary..."
cargo build --release --manifest-path (Join-Path $repoRoot "Cargo.toml")
if ($LASTEXITCODE -ne 0) {
    throw "cargo build failed with exit code $LASTEXITCODE"
}
if (!(Test-Path $binaryPath)) {
    throw "Missing release binary: $binaryPath"
}
if (!(Test-Path $logoPath)) {
    throw "Missing tray icon asset: $logoPath"
}
New-Item -ItemType Directory -Force $distRoot | Out-Null
New-Item -ItemType Directory -Force $assetDir | Out-Null
Copy-Item $binaryPath (Join-Path $distRoot "KeyEcho.exe") -Force
Copy-Item $configTemplate (Join-Path $distRoot "config.json") -Force
Copy-Item $logoPath (Join-Path $assetDir "logo.ico") -Force
Write-Host "Windows package output: $distRoot"
