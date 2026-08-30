param(
  [string]$Destination = (Join-Path (Get-Location) ("backups\" + (Get-Date -Format "yyyyMMdd-HHmmss")))
)
$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $Destination | Out-Null
$roots = @(
  (Join-Path $env:APPDATA 'ug.masterchef.restaurant'),
  (Join-Path $env:APPDATA 'ug.masterchef.ceo')
)
$found = 0
foreach ($root in $roots) {
  if (Test-Path $root) {
    $name = Split-Path $root -Leaf
    $target = Join-Path $Destination $name
    Copy-Item -Path $root -Destination $target -Recurse -Force
    $found++
  }
}
if ($found -eq 0) { throw 'No MasterChef application data directories were found. Do not treat this as a successful backup.' }
$manifest = [ordered]@{ createdAt=(Get-Date).ToUniversalTime().ToString('o'); machine=$env:COMPUTERNAME; directories=$found }
$manifest | ConvertTo-Json | Set-Content (Join-Path $Destination 'manifest.json') -Encoding UTF8
Write-Host "MasterChef backup created at $Destination"
