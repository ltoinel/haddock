# Setup script to download Python embeddable, Sherlock, and Tor
# Run this on Windows before building the Tauri app

$ErrorActionPreference = "Stop"

# === Python config ===
$PYTHON_VERSION = "3.12.10"
$PYTHON_ZIP = "python-$PYTHON_VERSION-embed-amd64.zip"
$PYTHON_URL = "https://www.python.org/ftp/python/$PYTHON_VERSION/$PYTHON_ZIP"
$PYTHON_SHA256 = "4acbed6dd1c744b0376e3b1cf57ce906f9dc9e95e68824584c8099a63025a3c3"
$SHERLOCK_VERSION = "0.16.0"

# === Tor config ===
$TOR_VERSION = "15.0.8"
$TOR_ARCHIVE = "tor-expert-bundle-windows-x86_64-$TOR_VERSION.tar.gz"
$TOR_URL = "https://dist.torproject.org/torbrowser/$TOR_VERSION/$TOR_ARCHIVE"
$TOR_SHA256 = "0f09e0502a1bb6e3a7389b773e20cf112083bf6f25c1786ed8acd4b86273ea18"

# === Paths ===
$DEST = "$PSScriptRoot\..\src-tauri\python-embed"
$TOR_DEST = "$PSScriptRoot\..\src-tauri\tor"

Write-Host "=== Haddock: Setting up embedded Python + Sherlock + Tor ===" -ForegroundColor Cyan

# ============================
# Python + Sherlock
# ============================

if (Test-Path $DEST) {
    Write-Host "Removing previous embedded Python..."
    Remove-Item -Recurse -Force $DEST
}

New-Item -ItemType Directory -Path $DEST | Out-Null

# Download Python embeddable
$zipPath = "$env:TEMP\$PYTHON_ZIP"
if (-not (Test-Path $zipPath)) {
    Write-Host "Downloading Python $PYTHON_VERSION embeddable..."
    Invoke-WebRequest -Uri $PYTHON_URL -OutFile $zipPath
}

# Verify SHA-256
Write-Host "Verifying Python checksum..."
$hash = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()
if ($hash -ne $PYTHON_SHA256) {
    Remove-Item $zipPath -Force
    throw "SHA-256 mismatch for Python download! Expected: $PYTHON_SHA256, Got: $hash"
}

Write-Host "Extracting Python..."
Expand-Archive -Path $zipPath -DestinationPath $DEST -Force

# Enable import site (required for pip)
$pthFile = Get-ChildItem "$DEST\python*._pth" | Select-Object -First 1
if ($pthFile) {
    Write-Host "Enabling site-packages in $($pthFile.Name)..."
    $content = Get-Content $pthFile.FullName
    $content = $content -replace "^#import site", "import site"
    $content += "Lib\site-packages"
    Set-Content $pthFile.FullName $content
}

# Download and install pip
$getPipPath = "$env:TEMP\get-pip.py"
if (-not (Test-Path $getPipPath)) {
    Write-Host "Downloading get-pip.py..."
    Invoke-WebRequest -Uri "https://bootstrap.pypa.io/get-pip.py" -OutFile $getPipPath
}

Write-Host "Installing pip..."
& "$DEST\python.exe" $getPipPath --no-warn-script-location
if ($LASTEXITCODE -ne 0) { throw "Failed to install pip" }

Write-Host "Installing setuptools..."
& "$DEST\python.exe" -m pip install setuptools --no-warn-script-location --quiet
if ($LASTEXITCODE -ne 0) { throw "Failed to install setuptools" }

Write-Host "Installing sherlock-project==$SHERLOCK_VERSION..."
& "$DEST\python.exe" -m pip install "sherlock-project==$SHERLOCK_VERSION" --no-warn-script-location --quiet
if ($LASTEXITCODE -ne 0) { throw "Failed to install sherlock-project" }

Write-Host "Verifying Sherlock installation..."
& "$DEST\python.exe" -m sherlock_project --version
if ($LASTEXITCODE -ne 0) { throw "Sherlock verification failed" }

# Clean up
Write-Host "Cleaning up Python packages..."
$cleanDirs = @(
    "$DEST\Lib\site-packages\pip",
    "$DEST\Lib\site-packages\setuptools",
    "$DEST\Lib\site-packages\wheel",
    "$DEST\Lib\site-packages\pkg_resources"
)
foreach ($dir in $cleanDirs) {
    if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
}
Get-ChildItem -Path $DEST -Recurse -Directory -Filter "__pycache__" | Remove-Item -Recurse -Force
Get-ChildItem -Path "$DEST\Lib\site-packages" -Directory -Filter "pip-*" | Remove-Item -Recurse -Force
Get-ChildItem -Path "$DEST\Lib\site-packages" -Directory -Filter "setuptools-*" | Remove-Item -Recurse -Force
Get-ChildItem -Path "$DEST\Lib\site-packages" -Directory -Filter "wheel-*" | Remove-Item -Recurse -Force

# ============================
# Tor Expert Bundle
# ============================

if (Test-Path $TOR_DEST) {
    Write-Host "Removing previous Tor..."
    Remove-Item -Recurse -Force $TOR_DEST
}

New-Item -ItemType Directory -Path $TOR_DEST | Out-Null

$torPath = "$env:TEMP\$TOR_ARCHIVE"
if (-not (Test-Path $torPath)) {
    Write-Host "Downloading Tor Expert Bundle $TOR_VERSION..."
    Invoke-WebRequest -Uri $TOR_URL -OutFile $torPath
}

Write-Host "Verifying Tor checksum..."
$torHash = (Get-FileHash -Path $torPath -Algorithm SHA256).Hash.ToLower()
if ($torHash -ne $TOR_SHA256) {
    Remove-Item $torPath -Force
    throw "SHA-256 mismatch for Tor download! Expected: $TOR_SHA256, Got: $torHash"
}

Write-Host "Extracting Tor..."
$torExtract = "$env:TEMP\tor-extract"
if (Test-Path $torExtract) { Remove-Item -Recurse -Force $torExtract }
New-Item -ItemType Directory -Path $torExtract | Out-Null
tar -xzf $torPath -C $torExtract

# Copy only the needed files
Copy-Item "$torExtract\tor\tor.exe" "$TOR_DEST\tor.exe"
Copy-Item "$torExtract\data\geoip" "$TOR_DEST\geoip"
Copy-Item "$torExtract\data\geoip6" "$TOR_DEST\geoip6"

# Clean up extract dir
Remove-Item -Recurse -Force $torExtract

Write-Host "Verifying Tor installation..."
& "$TOR_DEST\tor.exe" --version
if ($LASTEXITCODE -ne 0) { throw "Tor verification failed" }

# ============================
# Summary
# ============================

$pythonSize = (Get-ChildItem -Path $DEST -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB
$torSize = (Get-ChildItem -Path $TOR_DEST -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host ""
Write-Host "=== Done! ===" -ForegroundColor Green
Write-Host "Python + Sherlock: $([math]::Round($pythonSize, 1)) MB"
Write-Host "Tor:               $([math]::Round($torSize, 1)) MB"
Write-Host "Total:             $([math]::Round($pythonSize + $torSize, 1)) MB"
Write-Host ""
Write-Host "You can now build the app with: npm run tauri build"
