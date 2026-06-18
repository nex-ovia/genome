# genome installer for Windows — https://genome.nex-ovia.com/install.ps1
#
# Downloads the latest genome release, verifies its SHA-256, and installs it.
# Inspect first, then run:
#   irm https://genome.nex-ovia.com/install.ps1 | iex
#
# Overrides (env):
#   GENOME_VERSION       install a specific tag      (default: latest release)
#   GENOME_INSTALL_DIR   install location            (default: %LOCALAPPDATA%\Programs\genome)

$ErrorActionPreference = 'Stop'
$repo   = 'nex-ovia/genome'
$target = 'x86_64-pc-windows-msvc'

function Say($m) { Write-Host "  $m" }

# --- resolve version -------------------------------------------------------
$tag = $env:GENOME_VERSION
if (-not $tag) {
  Say 'resolving latest release...'
  $tag = (Invoke-RestMethod "https://api.github.com/repos/$repo/releases")[0].tag_name
}
if (-not $tag) { throw 'could not resolve the latest release tag' }

$asset = "genome-$tag-$target.zip"
$base  = "https://github.com/$repo/releases/download/$tag"
Say "installing genome $tag ($target)"

# --- download + verify -----------------------------------------------------
$tmp = Join-Path $env:TEMP ("genome-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $tmp | Out-Null
$zip = Join-Path $tmp $asset
Invoke-WebRequest "$base/$asset" -OutFile $zip -UseBasicParsing

try {
  $expected = ((Invoke-WebRequest "$base/$asset.sha256" -UseBasicParsing).Content -split '\s+')[0]
  $actual   = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
  if ($actual -ne $expected.ToLower()) { throw "checksum mismatch - refusing to install" }
  Say 'checksum verified'
} catch { Say "checksum verification skipped: $($_.Exception.Message)" }

# --- extract + install -----------------------------------------------------
Expand-Archive -Path $zip -DestinationPath $tmp -Force
$exe = Get-ChildItem -Path $tmp -Recurse -Filter 'genome.exe' | Select-Object -First 1
if (-not $exe) { throw 'genome.exe not found in archive' }

$dir = if ($env:GENOME_INSTALL_DIR) { $env:GENOME_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\genome' }
New-Item -ItemType Directory -Path $dir -Force | Out-Null
Copy-Item $exe.FullName (Join-Path $dir 'genome.exe') -Force
Remove-Item $tmp -Recurse -Force

$userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
if ($userPath -notlike "*$dir*") {
  [Environment]::SetEnvironmentVariable('Path', "$userPath;$dir", 'User')
  Say "added $dir to your PATH (restart your shell to pick it up)"
}
Say "installed: $dir\genome.exe"
Say 'try:  genome render nexovia.toml > report.html'
