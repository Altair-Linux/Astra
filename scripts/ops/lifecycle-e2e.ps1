param(
  [Parameter(Mandatory = $true)]
  [string]$WorkspaceRoot,
  [string]$AstraDir = "Astra",
  [string]$PackagesDir = "packages",
  [string]$AltairRepoDir = "altair-repo",
  [string[]]$PackageList = @("nano", "curl", "htop", "jq", "ripgrep", "tree"),
  [string]$BindAddress = "127.0.0.1:18081",
  [string]$RunId = "local"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Log {
  param([string]$Message)
  $ts = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
  Write-Host "[$ts] $Message"
}

function Invoke-Step {
  param(
    [string]$Name,
    [scriptblock]$Action
  )
  Write-Log "START: $Name"
  & $Action
  if ($LASTEXITCODE -ne 0) {
    throw "Step failed: $Name (exit code $LASTEXITCODE)"
  }
  Write-Log "OK: $Name"
}

function Get-AstraBinaryPath {
  param([string]$AstraRepo)
  $win = Join-Path $AstraRepo "target/debug/astra.exe"
  $unix = Join-Path $AstraRepo "target/debug/astra"
  if (Test-Path $win) { return $win }
  if (Test-Path $unix) { return $unix }
  throw "Astra binary not found in $AstraRepo/target/debug"
}

function Get-PackageMetaFromFilename {
  param([string]$FileName)
  $stem = [System.IO.Path]::GetFileNameWithoutExtension($FileName)
  if ($stem -notmatch '^(?<name>.+)-(?<version>[^-]+)-(?<arch>[^-]+)$') {
    throw "Invalid package artifact filename: $FileName"
  }
  [PSCustomObject]@{
    name = $Matches['name']
    version = $Matches['version']
    architecture = $Matches['arch']
  }
}

$resolvedRoot = (Resolve-Path $WorkspaceRoot).Path
$astraRepo = Join-Path $resolvedRoot $AstraDir
$packagesRepo = Join-Path $resolvedRoot $PackagesDir
$altairRepo = Join-Path $resolvedRoot $AltairRepoDir

$opsRoot = Join-Path $resolvedRoot (".ops-runtime/{0}" -f $RunId)
$dataDir = Join-Path $opsRoot "astra-data"
$rootDir = Join-Path $opsRoot "astra-root"
$repoRoot = Join-Path $opsRoot "repo/unstable"
$repoPackagesDir = Join-Path $repoRoot "packages"
$repoKeysDir = Join-Path $repoRoot "keys"
$indexPath = Join-Path $repoRoot "index.json"
$logsDir = Join-Path $opsRoot "logs"
$serverOut = Join-Path $logsDir "serve-repo.out.log"
$serverErr = Join-Path $logsDir "serve-repo.err.log"
$lifecycleReport = Join-Path $opsRoot "lifecycle-report.json"

New-Item -ItemType Directory -Force -Path $opsRoot, $logsDir, $repoPackagesDir, $repoKeysDir | Out-Null

$transcript = Join-Path $logsDir "lifecycle-transcript.log"
Start-Transcript -Path $transcript -Append | Out-Null

$serverProc = $null

try {
  Invoke-Step "Build Astra" {
    Push-Location $astraRepo
    cargo build -p astra
    Pop-Location
  }

  $astraBin = Get-AstraBinaryPath -AstraRepo $astraRepo

  foreach ($p in @($dataDir, $rootDir, $repoRoot)) {
    if (Test-Path $p) {
      Remove-Item -Recurse -Force $p
    }
  }
  New-Item -ItemType Directory -Force -Path $repoPackagesDir, $repoKeysDir | Out-Null

  Invoke-Step "Initialize Astra test environment" {
    & $astraBin --data-dir $dataDir --root $rootDir init
  }

  Invoke-Step "Generate CI signing key" {
    & $astraBin --data-dir $dataDir --root $rootDir key generate
  }

  $repoPubKey = Join-Path $repoKeysDir "repo.pub"
  Invoke-Step "Export repo public key" {
    & $astraBin --data-dir $dataDir --root $rootDir key export -o $repoPubKey
  }

  Invoke-Step "Import trusted repo key" {
    & $astraBin --data-dir $dataDir --root $rootDir key import localrepo $repoPubKey
  }

  $artifactRows = @()
  $indexEntries = @()

  foreach ($pkg in $PackageList) {
    $pkgPath = Join-Path $packagesRepo $pkg
    if (-not (Test-Path $pkgPath)) {
      throw "Package directory missing: $pkgPath"
    }

    Invoke-Step "Build package $pkg" {
      & $astraBin --data-dir $dataDir --root $rootDir build $pkgPath --output $repoPackagesDir
    }

    $artifact = Get-ChildItem -Path $repoPackagesDir -Filter "$pkg-*.astpkg" -File |
      Sort-Object LastWriteTime -Descending |
      Select-Object -First 1

    if (-not $artifact) {
      throw "Artifact missing for package $pkg"
    }

    $checksum = (Get-FileHash -Path $artifact.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    $meta = Get-PackageMetaFromFilename -FileName $artifact.Name

    $artifactRows += [PSCustomObject]@{
      package = $meta.name
      artifact = $artifact.Name
      size = $artifact.Length
      checksum = $checksum
    }

    $indexEntries += [PSCustomObject]@{
      name = $meta.name
      version = $meta.version
      architecture = $meta.architecture
      description = ""
      dependencies = @()
      conflicts = @()
      provides = @()
      checksum = $checksum
      filename = $artifact.Name
      size = $artifact.Length
      license = ""
      maintainer = ""
    }

    Write-Log ("artifact: {0} ({1} bytes)" -f $artifact.Name, $artifact.Length)
  }

  $index = [ordered]@{
    name = "unstable"
    description = "Altair Linux package repository"
    last_updated = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
    packages = $indexEntries
  }
  $index | ConvertTo-Json -Depth 10 | Set-Content -Path $indexPath -NoNewline

  $parsed = Get-Content -Raw $indexPath | ConvertFrom-Json
  if ($parsed.name -ne "unstable") {
    throw "index.json validation failed: unexpected repository name"
  }

  $indexPackageCount = ($parsed.packages | Measure-Object).Count
  if ($indexPackageCount -ne $PackageList.Count) {
    throw "index.json validation failed: expected $($PackageList.Count) packages, got $indexPackageCount"
  }

  foreach ($entry in $parsed.packages) {
    $pkgFile = Join-Path $repoPackagesDir $entry.filename
    if (-not (Test-Path $pkgFile)) {
      throw "index.json validation failed: missing file $($entry.filename)"
    }
    $actualHash = (Get-FileHash -Path $pkgFile -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actualHash -ne $entry.checksum) {
      throw "index.json validation failed: checksum mismatch for $($entry.filename)"
    }
  }

  Write-Log "Starting repository server"
  $serverProc = Start-Process -FilePath $astraBin -WorkingDirectory $resolvedRoot -ArgumentList @(
    "--data-dir", $dataDir,
    "--root", $rootDir,
    "serve-repo", $repoRoot,
    "--bind", $BindAddress
  ) -PassThru -RedirectStandardOutput $serverOut -RedirectStandardError $serverErr

  $uri = "http://$BindAddress/index.json"
  $ready = $false
  for ($i = 1; $i -le 60; $i++) {
    Start-Sleep -Seconds 1
    try {
      $resp = Invoke-WebRequest -Uri $uri -UseBasicParsing -TimeoutSec 2
      if ($resp.StatusCode -eq 200) {
        $ready = $true
        Write-Log "Repository server ready after $i attempts"
        break
      }
    } catch {
      if (($i % 10) -eq 0) {
        Write-Log "Readiness probe retry $i/60"
      }
    }
  }

  if (-not $ready) {
    Write-Log "Repository server readiness failed"
    if (Test-Path $serverOut) { Get-Content -Path $serverOut -Tail 120 | Write-Host }
    if (Test-Path $serverErr) { Get-Content -Path $serverErr -Tail 120 | Write-Host }
    throw "serve-repo failed readiness probe"
  }

  try {
    Invoke-Step "Configure temporary repository" {
      & $astraBin --data-dir $dataDir --root $rootDir repo remove unstable-local
    }
  } catch {
    Write-Log "No existing unstable-local repo to remove (continuing)"
  }

  Invoke-Step "Add temporary repository" {
    & $astraBin --data-dir $dataDir --root $rootDir repo add unstable-local ("http://{0}/" -f $BindAddress)
  }

  Invoke-Step "Update repository metadata" {
    & $astraBin --data-dir $dataDir --root $rootDir update
  }

  foreach ($pkg in $PackageList) {
    Invoke-Step "Install $pkg" {
      & $astraBin --data-dir $dataDir --root $rootDir install $pkg
    }
    Invoke-Step "Remove $pkg" {
      & $astraBin --data-dir $dataDir --root $rootDir remove $pkg
    }
    Invoke-Step "Upgrade after $pkg" {
      & $astraBin --data-dir $dataDir --root $rootDir upgrade
    }
  }

  $report = [ordered]@{
    status = "success"
    workspace = $resolvedRoot
    run_id = $RunId
    bind_address = $BindAddress
    artifacts = $artifactRows
    index_path = $indexPath
    logs = [ordered]@{
      transcript = $transcript
      server_stdout = $serverOut
      server_stderr = $serverErr
    }
    generated_at = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  }
  $report | ConvertTo-Json -Depth 12 | Set-Content -Path $lifecycleReport -NoNewline
  Write-Log "Lifecycle E2E completed successfully"
  Write-Log "Report: $lifecycleReport"
}
catch {
  Write-Log ("FAILURE: {0}" -f $_.Exception.Message)
  throw
}
finally {
  if ($serverProc -and -not $serverProc.HasExited) {
    Stop-Process -Id $serverProc.Id -Force
  }
  Stop-Transcript | Out-Null
}
