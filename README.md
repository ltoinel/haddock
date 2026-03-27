# Haddock

Haddock is a Windows desktop application that provides a clean, user-friendly interface for [Sherlock OSINT](https://github.com/sherlock-project/sherlock), the popular username search tool. It allows anyone to search for a username across 400+ social networks without needing to install Python, Tor, or use a command line.

Built with [Tauri](https://tauri.app/) (Rust + TypeScript), Haddock embeds Python, Sherlock, and Tor directly in the installer — zero external dependencies.

## Features

- **Zero dependencies** — Python, Sherlock, and Tor are embedded in the application
- **Multi-username search** — search several usernames at once (space separated)
- **Real-time results** — found accounts appear as they are discovered in a card grid
- **Tor anonymization** — built-in Tor support with one-click toggle and live status indicator
- **Advanced options** — timeout, proxy, specific sites, NSFW filter, show all sites
- **One-click profile access** — click any result to open it in your default browser
- **Filter & copy** — filter results in real-time, copy all found URLs to clipboard
- **Debug console** — always-on bottom sheet console showing stdout/stderr/Tor bootstrap logs
- **Cancel anytime** — stop a running search or Tor connection instantly
- **Lightweight** — native Tauri app, no Electron, minimal resource usage

## Installation

Download the latest installer from the [Releases](../../releases) page:

- **`.exe`** — NSIS installer (recommended, user-level install, no admin required)

## Build from source

### Prerequisites

- [Node.js](https://nodejs.org/) >= 20
- [Rust](https://rustup.rs/) (stable)
- Windows (for the embedded Python/Tor setup and final build)

### Steps

```powershell
# Clone the repository
git clone https://github.com/ltoinel/haddock.git
cd haddock

# Install npm dependencies
npm install

# Download and setup embedded Python + Sherlock + Tor
.\scripts\setup-python.ps1

# Build the installer
npx tauri build
```

The installer is generated in `src-tauri/target/release/bundle/nsis/`.

### Development

```powershell
# Make sure embedded Python + Tor are set up first
.\scripts\setup-python.ps1

# Start dev mode with hot reload
npx tauri dev
```

## How it works

1. The build script (`scripts/setup-python.ps1`) downloads:
   - [Python embeddable package](https://www.python.org/downloads/) (3.12.10) with SHA-256 verification
   - [sherlock-project](https://pypi.org/project/sherlock-project/) (0.16.0) via pip
   - [Tor Expert Bundle](https://www.torproject.org/download/tor/) (15.0.8) with SHA-256 verification
2. Tauri bundles `python-embed/` and `tor/` directories as resources inside the installer.
3. At runtime, the Rust backend:
   - Locates the embedded `python.exe` and runs Sherlock as a child process (hidden, no console window)
   - Optionally starts `tor.exe`, waits for bootstrap, then passes `--tor` to Sherlock
   - Streams output line-by-line to the frontend via Tauri events for real-time display
   - Kills child processes (Sherlock, Tor) on cancel, timeout, or app close

## CI / CD

- **CI** (`ci.yml`) — runs on every push/PR: TypeScript check, Clippy lint, Rust tests
- **Release** (`build.yml`) — runs when a GitHub Release is created: full build on `windows-latest`, attaches the `.exe` installer to the release

### Creating a release

1. Go to **Releases** > **Draft a new release**
2. Create a tag (e.g., `v0.1.0`), add a title, then **Publish release**
3. The build workflow runs automatically and attaches the installer

## License

This project is licensed under the **MIT License** — see [LICENSE](LICENSE) for details.

Haddock bundles third-party software. See [THIRD-PARTY-LICENSES](THIRD-PARTY-LICENSES) for the full license texts of all embedded components.

## Credits

- [Sherlock Project](https://github.com/sherlock-project/sherlock) — the OSINT engine powering username searches
- [Tauri](https://tauri.app/) — the framework used to build this native app
- [Python](https://www.python.org/) — embedded runtime for Sherlock
- [Tor Project](https://www.torproject.org/) — embedded anonymization network
