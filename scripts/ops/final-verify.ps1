param(
  [Parameter(Mandatory = $true)]
  [string]$WorkspaceRoot,
  [string[]]$Repositories = @("Altair-Linux/Astra", "Altair-Linux/packages", "Altair-Linux/altair-repo"),
  [string[]]$PackageList = @("nano", "curl", "htop", "jq", "ripgrep", "tree"),
  [string]$AltairRepoPath = "altair-repo",
  [switch]$RunLifecycle,
  [string]$LifecycleRunId = "final-verify"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$root = (Resolve-Path $WorkspaceRoot).Path
$lifecycleScript = Join-Path $root "Astra/scripts/ops/lifecycle-e2e.ps1"
$reportPath = Join-Path $root ".ops-runtime/$LifecycleRunId/lifecycle-report.json"

if ($RunLifecycle) {
  & $lifecycleScript -WorkspaceRoot $root -RunId $LifecycleRunId
}

if (-not (Test-Path $reportPath)) {
  throw "Lifecycle report missing: $reportPath"
}

$report = Get-Content -Raw $reportPath | ConvertFrom-Json
if ($report.status -ne "success") {
  throw "Lifecycle status is not success"
}

$artifactMap = @{}
foreach ($a in $report.artifacts) {
  $artifactMap[$a.package] = $true
}
foreach ($pkg in $PackageList) {
  if (-not $artifactMap.ContainsKey($pkg)) {
    throw "Lifecycle report missing artifact for package: $pkg"
  }
}

$altairRepo = Join-Path $root $AltairRepoPath
$unstableIndex = Join-Path $altairRepo "unstable/index.json"
$unstablePackagesDir = Join-Path $altairRepo "unstable/packages"

if (-not (Test-Path $unstableIndex)) {
  throw "Altair repo index missing: $unstableIndex"
}
if (-not (Test-Path $unstablePackagesDir)) {
  throw "Altair repo packages dir missing: $unstablePackagesDir"
}

$index = Get-Content -Raw $unstableIndex | ConvertFrom-Json
foreach ($entry in $index.packages) {
  $pkgFile = Join-Path $unstablePackagesDir $entry.filename
  if (-not (Test-Path $pkgFile)) {
    throw "Index references missing package file: $($entry.filename)"
  }

  $actualChecksum = (Get-FileHash -Path $pkgFile -Algorithm SHA256).Hash.ToLowerInvariant()
  if ($actualChecksum -ne $entry.checksum) {
    throw "Checksum mismatch for $($entry.filename)"
  }
}

$repoChecks = @()
foreach ($repo in $Repositories) {
  $main = gh api "repos/$repo/commits/main" | ConvertFrom-Json
  $openPrs = gh pr list -R $repo --state open --limit 50 --json number | ConvertFrom-Json
  $workflowResp = gh api "repos/$repo/actions/workflows?per_page=100" | ConvertFrom-Json
  $workflows = $workflowResp.workflows | Select-Object name,state,path

  if (-not $main.commit.verification.verified) {
    throw "Repository $repo has unverified main head ($($main.commit.verification.reason))"
  }
  if (($openPrs | Measure-Object).Count -ne 0) {
    throw "Repository $repo still has open PRs"
  }

  $repoChecks += [PSCustomObject]@{
    repository = $repo
    main_sha = $main.sha
    verified = $main.commit.verification.verified
    verification_reason = $main.commit.verification.reason
    latest_message = $main.commit.message
    workflows = $workflows
  }
}

Write-Host "\n=== Final Closure Report ==="
$repoChecks | Format-Table -AutoSize
Write-Host "\nLifecycle report: $reportPath"
$report.artifacts | Select-Object package,artifact,size | Format-Table -AutoSize

[ordered]@{
  status = "success"
  generated_at = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
  repositories = $repoChecks
  lifecycle_report = $reportPath
} | ConvertTo-Json -Depth 8
