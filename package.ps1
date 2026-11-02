$ErrorActionPreference = "Stop"
$repoRoot = $PSScriptRoot
$distRoot = Join-Path $repoRoot "TempFile\dist\windows"
$assetDir = Join-Path $distRoot "asset"
$binaryPath = Join-Path $repoRoot "TempFile\target\release\KeyEcho.exe"
$configTemplate = Join-Path $repoRoot "asset\config.template.json"
$localeExample = Join-Path $repoRoot "asset\locale.example.json"
Write-Host "Building release binary..."
Push-Location -LiteralPath $repoRoot
try {
    # Cargo discovers .cargo/config.toml from the working directory.
    cargo build --release --manifest-path (Join-Path $repoRoot "Cargo.toml")
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
    }
}
finally {
    Pop-Location
}
if (!(Test-Path $binaryPath)) {
    throw "Missing release binary: $binaryPath"
}
New-Item -ItemType Directory -Force $distRoot | Out-Null
New-Item -ItemType Directory -Force $assetDir | Out-Null
Copy-Item $binaryPath (Join-Path $distRoot "KeyEcho.exe") -Force
Copy-Item $configTemplate (Join-Path $distRoot "config.json") -Force
Copy-Item -LiteralPath $localeExample -Destination (Join-Path $distRoot "locale.example.json") -Force
# 清理旧版打包遗留的外部图标，音频文件保留。
$legacyLogoPath = Join-Path $assetDir "logo.ico"
if (Test-Path -LiteralPath $legacyLogoPath) {
    Remove-Item -LiteralPath $legacyLogoPath -Force
}
Write-Host "Windows package output: $distRoot"
