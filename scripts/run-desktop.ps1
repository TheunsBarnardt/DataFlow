$ErrorActionPreference = "Stop"
$env:CARGO_HTTP_CHECK_REVOKE = "false"
if (-not $env:HOST) { $env:HOST = "127.0.0.1" }
if (-not $env:PORT) { $env:PORT = "8788" }
if (-not $env:BOTFLOW_AUTH_DISABLED) { $env:BOTFLOW_AUTH_DISABLED = "1" }
if (-not $env:HTTP_ALLOW_PRIVATE) { $env:HTTP_ALLOW_PRIVATE = "true" }
if (-not $env:MONGODB_URI) { $env:MONGODB_URI = "mongodb://127.0.0.1:27017/botflow" }

$root = Split-Path -Parent $PSScriptRoot
$useHotPatch = $env:BOTFLOW_HOT_PATCH -eq "1"
# Default: full app restart on every change (so version bumps / Rust edits are visible).
# Set BOTFLOW_HOT_RELOAD=1 to keep RSX-only hot reload instead.
$useHotReload = $env:BOTFLOW_HOT_RELOAD -eq "1"

Write-Host "Building Tailwind for Dioxus studio..."
Push-Location $root
npx --yes @tailwindcss/cli@4 -i ./crates/desktop/tailwind.css -o ./crates/desktop/assets/tailwind.css

$dx = Get-Command dx -ErrorAction SilentlyContinue
if (-not $dx) {
    Write-Host "dioxus-cli (dx) not found. Installing..."
    cargo install dioxus-cli --locked
}

Push-Location (Join-Path $root "crates\desktop")
if ($useHotPatch) {
    Write-Host "Starting BotFlow desktop with RSX + Rust hot-patch (dx serve --desktop --hot-patch)..."
    Write-Host "Note: --hot-patch needs MSVC lib paths (run from 'x64 Native Tools' if link fails)."
    dx serve --desktop --hot-patch -p botflow-desktop
} elseif ($useHotReload) {
    Write-Host "Starting BotFlow desktop with RSX hot reload (dx serve --desktop)..."
    Write-Host "Tip: omit BOTFLOW_HOT_RELOAD for full auto-restart on every change."
    dx serve --desktop -p botflow-desktop
} else {
    Write-Host "Starting BotFlow desktop with auto-restart on every change (dx serve --desktop --hot-reload false)..."
    Write-Host "Tip: set BOTFLOW_HOT_RELOAD=1 for RSX-only hot reload, or BOTFLOW_HOT_PATCH=1 for Rust hot-patch."
    dx serve --desktop --hot-reload false -p botflow-desktop
}
Pop-Location
Pop-Location
