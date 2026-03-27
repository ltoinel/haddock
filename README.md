# Haddock

Haddock is a Windows desktop application that provides a clean, user-friendly interface for [Sherlock OSINT](https://github.com/sherlock-project/sherlock), the popular username search tool. It allows anyone to search for a username across 400+ social networks without needing to install Python or use a command line.

Built with [Tauri](https://tauri.app/) (Rust + TypeScript), Haddock embeds a portable Python runtime and Sherlock directly in the installer — no external dependencies required.

## Features

- **Zero dependencies** — Python and Sherlock are embedded in the application
- **Real-time results** — found accounts appear as they are discovered
- **One-click profile access** — click any result to open it in your browser
- **Cancel anytime** — stop a running search instantly
- **Lightweight** — native Tauri app with minimal resource usage
- **Clean dark UI** — simple, distraction-free interface

## Installation

Download the latest installer from the [Releases](../../releases) page:

- **`.exe`** — NSIS installer (recommended, user-level install)
- **`.msi`** — MSI installer (alternative)

## Build from source

### Prerequisites

- [Node.js](https://nodejs.org/) >= 20
- [Rust](https://rustup.rs/) (stable)
- Windows (for the embedded Python setup and final build)

### Steps

```powershell
# Clone the repository
git clone https://github.com/ltoinel/haddock.git
cd haddock

# Install npm dependencies
npm install

# Download and setup embedded Python + Sherlock
.\scripts\setup-python.ps1

# Build the installer
npx tauri build
```

The installers are generated in `src-tauri/target/release/bundle/`.

### Development

```powershell
# Make sure embedded Python is set up first
.\scripts\setup-python.ps1

# Start dev mode with hot reload
npx tauri dev
```

## How it works

1. The build script (`scripts/setup-python.ps1`) downloads the official [Python embeddable package](https://www.python.org/downloads/) for Windows and installs [sherlock-project](https://pypi.org/project/sherlock-project/) into it.
2. Tauri bundles this `python-embed` directory as a resource inside the installer.
3. At runtime, the Rust backend locates the embedded `python.exe` and runs Sherlock as a child process.
4. Output is streamed line-by-line to the frontend via Tauri events for real-time display.

## Creating a release

Push a version tag to trigger the GitHub Actions workflow and automatically create a release with installers attached:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## License

This project is licensed under the **MIT License** — see [LICENSE](LICENSE) for details.

Haddock bundles third-party software. See [THIRD-PARTY-LICENSES](THIRD-PARTY-LICENSES) for the full license texts of all embedded components.

## Credits

- [Sherlock Project](https://github.com/sherlock-project/sherlock) — the OSINT engine powering username searches
- [Tauri](https://tauri.app/) — the framework used to build this native app
- [Python](https://www.python.org/) — embedded runtime for Sherlock
