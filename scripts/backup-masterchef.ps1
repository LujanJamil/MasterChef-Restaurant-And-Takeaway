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
$files = @()
foreach ($root in $roots) {
  if (Test-Path $root) {
    $name = Split-Path $root -Leaf
    $target = Join-Path $Destination $name
    Copy-Item -Path $root -Destination $target -Recurse -Force
    $found++
    $files += Get-ChildItem -Path $target -File -Recurse | ForEach-Object {
      $h = Get-FileHash -Algorithm SHA256 -Path $_.FullName
      [ordered]@{ path=$h.Path.Substring($Destination.Length).TrimStart('\'); sha256=$h.Hash; bytes=$h.Length }
    }
  }
}
if ($found -eq 0) { throw 'No MasterChef application data directories were found. Do not treat this as a successful backup.' }
$manifest = [ordered]@{
  schemaVersion=1
  createdAt=(Get-Date).ToUniversalTime().ToString('o')
  machine=$env:COMPUTERNAME
  directories=$found
  files=$files
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $Destination 'manifest.json') -Encoding UTF8
$archive = "$Destination.zip"
Compress-Archive -Path (Join-Path $Destination '*') -DestinationPath $archive -Force
$archiveHash = Get-FileHash -Algorithm SHA256 -Path $archive
[ordered]@{ archive=$archive; sha256=$archiveHash.Hash; createdAt=$manifest.createdAt } | ConvertTo-Json | Set-Content (Join-Path $Destination 'archive-checksum.json') -Encoding UTF8
Write-Host "MasterChef backup created at $Destination"
Write-Host "Archive: $archive"
Write-Host "SHA256: $($archiveHash.Hash)"
