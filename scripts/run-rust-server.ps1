$ErrorActionPreference = "Stop"
$env:CARGO_HTTP_CHECK_REVOKE = "false"
$env:HOST = if ($env:HOST) { $env:HOST } else { "127.0.0.1" }
$env:PORT = if ($env:PORT) { $env:PORT } else { "8788" }
$env:MONGODB_URI = if ($env:MONGODB_URI) { $env:MONGODB_URI } else { "mongodb://127.0.0.1:27017/botflow" }
$env:BOTFLOW_AUTH_DISABLED = if ($env:BOTFLOW_AUTH_DISABLED) { $env:BOTFLOW_AUTH_DISABLED } else { "1" }
$env:HTTP_ALLOW_PRIVATE = if ($env:HTTP_ALLOW_PRIVATE) { $env:HTTP_ALLOW_PRIVATE } else { "true" }
Write-Host "Starting botflow-server on $($env:HOST):$($env:PORT)"
cargo run -p botflow-app
