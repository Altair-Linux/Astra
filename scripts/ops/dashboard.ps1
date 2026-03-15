param(
  [string[]]$Repositories = @("Altair-Linux/Astra", "Altair-Linux/packages", "Altair-Linux/altair-repo"),
  [string]$LifecycleReportPath,
  [switch]$IncludeWorkflows = $true,
  [switch]$IncludeOpenPrs = $true
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$rows = @()
foreach ($repo in $Repositories) {
  $main = gh api "repos/$repo/commits/main" | ConvertFrom-Json
  $openPrCount = 0
  if ($IncludeOpenPrs) {
    $prs = gh pr list -R $repo --state open --limit 100 --json number,title | ConvertFrom-Json
    $openPrCount = ($prs | Measure-Object).Count
  }

  $workflowCount = 0
  if ($IncludeWorkflows) {
    $w = gh api "repos/$repo/actions/workflows?per_page=100" | ConvertFrom-Json
    $workflowCount = ($w.workflows | Measure-Object).Count
  }

  $rows += [PSCustomObject]@{
    repository = $repo
    main_sha = $main.sha
    verified = $main.commit.verification.verified
    verification_reason = $main.commit.verification.reason
    open_prs = $openPrCount
    workflows = $workflowCount
    latest_message = $main.commit.message
  }
}

Write-Host "\n=== Ecosystem Status Dashboard ==="
$rows | Format-Table -AutoSize

if ($LifecycleReportPath -and (Test-Path $LifecycleReportPath)) {
  Write-Host "\n=== Lifecycle Report Summary ==="
  $lifecycle = Get-Content -Raw $LifecycleReportPath | ConvertFrom-Json
  Write-Host ("status: {0}" -f $lifecycle.status)
  if ($lifecycle.artifacts) {
    $lifecycle.artifacts | Select-Object package,artifact,size | Format-Table -AutoSize
  }
}

$rows | ConvertTo-Json -Depth 6
