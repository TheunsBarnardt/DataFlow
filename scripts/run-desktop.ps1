$ErrorActionPreference = "Stop"
$env:CARGO_HTTP_CHECK_REVOKE = "false"
if (-not $env:HOST) { $env:HOST = "127.0.0.1" }
if (-not $env:PORT) { $env:PORT = "8788" }
if (-not $env:BOTFLOW_AUTH_DISABLED) { $env:BOTFLOW_AUTH_DISABLED = "1" }
if (-not $env:HTTP_ALLOW_PRIVATE) { $env:HTTP_ALLOW_PRIVATE = "true" }
if (-not $env:MONGODB_URI) { $env:MONGODB_URI = "mongodb://127.0.0.1:27017/botflow" }

$root = Split-Path -Parent $PSScriptRoot
$useHotPatch = $env:BOTFLOW_HOT_PATCH -eq "1"

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
} else {
    Write-Host "Starting BotFlow desktop with RSX hot reload (dx serve --desktop)..."
    Write-Host "Tip: set BOTFLOW_HOT_PATCH=1 for experimental Rust logic hot-patching."
    dx serve --desktop -p botflow-desktop
}
Pop-Location
Pop-Location
