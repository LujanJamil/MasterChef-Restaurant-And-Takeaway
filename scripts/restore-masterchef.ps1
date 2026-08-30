param([Parameter(Mandatory=$true)][string]$BackupDirectory)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path $BackupDirectory)) { throw "Backup directory not found: $BackupDirectory" }
$manifest = Join-Path $BackupDirectory 'manifest.json'
if (-not (Test-Path $manifest)) { throw 'Backup manifest is missing; restore refused.' }
$targets = @(
  @{Name='ug.masterchef.restaurant'; Path=(Join-Path $env:APPDATA 'ug.masterchef.restaurant')},
  @{Name='ug.masterchef.ceo'; Path=(Join-Path $env:APPDATA 'ug.masterchef.ceo')}
)
foreach ($target in $targets) {
  $source = Join-Path $BackupDirectory $target.Name
  if (Test-Path $source) {
    if (Test-Path $target.Path) { Rename-Item $target.Path ($target.Path + '.pre-restore-' + (Get-Date -Format 'yyyyMMdd-HHmmss')) }
    Copy-Item $source $target.Path -Recurse -Force
  }
}
Write-Host 'MasterChef restore completed. Start the applications and perform the runtime acceptance checklist before trusting the restored state.'
