# Setup script to download Python embeddable and install Sherlock into it
# Run this on Windows before building the Tauri app

$ErrorActionPreference = "Stop"

$PYTHON_VERSION = "3.12.9"
$PYTHON_ZIP = "python-$PYTHON_VERSION-embed-amd64.zip"
$PYTHON_URL = "https://www.python.org/ftp/python/$PYTHON_VERSION/$PYTHON_ZIP"
$DEST = "$PSScriptRoot\..\src-tauri\python-embed"

Write-Host "=== Haddock: Setting up embedded Python + Sherlock ===" -ForegroundColor Cyan

# Clean previous install
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

# Extract
Write-Host "Extracting Python..."
Expand-Archive -Path $zipPath -DestinationPath $DEST -Force

# Enable import site (required for pip)
$pthFile = Get-ChildItem "$DEST\python*._pth" | Select-Object -First 1
if ($pthFile) {
    Write-Host "Enabling site-packages in $($pthFile.Name)..."
    $content = Get-Content $pthFile.FullName
    $content = $content -replace "^#import site", "import site"
    # Also add Lib\site-packages to the path
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

# Install setuptools (required to build some Sherlock dependencies)
Write-Host "Installing setuptools..."
& "$DEST\python.exe" -m pip install setuptools --no-warn-script-location --quiet
if ($LASTEXITCODE -ne 0) { throw "Failed to install setuptools" }

# Install sherlock-project with all dependencies using only prebuilt wheels
Write-Host "Installing sherlock-project..."
& "$DEST\python.exe" -m pip install sherlock-project --no-warn-script-location --quiet
if ($LASTEXITCODE -ne 0) { throw "Failed to install sherlock-project" }

# Verify sherlock is importable
Write-Host "Verifying Sherlock installation..."
& "$DEST\python.exe" -m sherlock_project --version
if ($LASTEXITCODE -ne 0) { throw "Sherlock verification failed" }

# Clean up unnecessary files to reduce bundle size
Write-Host "Cleaning up to reduce size..."
$cleanDirs = @(
    "$DEST\Lib\site-packages\pip",
    "$DEST\Lib\site-packages\setuptools",
    "$DEST\Lib\site-packages\wheel",
    "$DEST\Lib\site-packages\pkg_resources"
)
foreach ($dir in $cleanDirs) {
    if (Test-Path $dir) {
        Remove-Item -Recurse -Force $dir
    }
}

# Remove __pycache__ directories
Get-ChildItem -Path $DEST -Recurse -Directory -Filter "__pycache__" | Remove-Item -Recurse -Force

# Remove .dist-info directories for cleaned packages
Get-ChildItem -Path "$DEST\Lib\site-packages" -Directory -Filter "pip-*" | Remove-Item -Recurse -Force
Get-ChildItem -Path "$DEST\Lib\site-packages" -Directory -Filter "setuptools-*" | Remove-Item -Recurse -Force
Get-ChildItem -Path "$DEST\Lib\site-packages" -Directory -Filter "wheel-*" | Remove-Item -Recurse -Force

$size = (Get-ChildItem -Path $DEST -Recurse | Measure-Object -Property Length -Sum).Sum / 1MB
Write-Host ""
Write-Host "=== Done! ===" -ForegroundColor Green
Write-Host "Embedded Python + Sherlock installed at: $DEST"
Write-Host "Total size: $([math]::Round($size, 1)) MB"
Write-Host ""
Write-Host "You can now build the app with: npm run tauri build"
