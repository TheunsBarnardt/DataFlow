# Extract request schemas from original ASP.NET APIs and regenerate DataFlow mirrors.
# Usage: pwsh scripts/sync-api-request-schemas.ps1
$ErrorActionPreference = "Stop"
Set-Location (Split-Path $PSScriptRoot -Parent)

Write-Host "1/3 Extract from C# projects..."
python scripts/extract-request-schemas.py
Write-Host "2/3 Generate TypeScript module..."
python scripts/generate-request-schemas-ts.py
Write-Host "3/3 Regenerate Online/Dynamics catalogs from extract..."
python scripts/generate-api-catalogs.py
Write-Host "Done. Restart DataFlow server to surgically reapply params onto existing workflows."
