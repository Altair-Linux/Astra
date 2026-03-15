param(
  [Parameter(Mandatory = $true)]
  [string]$Repository,
  [Parameter(Mandatory = $true)]
  [string]$RepositoryPath,
  [string]$Branch = "finalize-ecosystem",
  [string]$CommitMessage = "chore: automated verified checkpoint",
  [string]$PrTitle = "Finalize ecosystem",
  [string]$PrBody = "Automated PR with signed commit and verified merge.",
  [ValidateSet("squash", "rebase")]
  [string]$MergeMethod = "squash",
  [switch]$AllowEmptyCommit,
  [switch]$UseAdmin,
  [switch]$ToggleRuleset
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Log {
  param([string]$Message)
  $ts = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
  Write-Host "[$ts] $Message"
}

function Invoke-RulesetToggle {
  param([string]$State)
  $scriptPath = Join-Path $PSScriptRoot "toggle-ruleset.ps1"
  & $scriptPath -Repository $Repository -Enforcement $State
}

$repoPath = (Resolve-Path $RepositoryPath).Path

Push-Location $repoPath
try {
  git checkout main

  $hasChanges = ((git status --porcelain) | Measure-Object).Count -gt 0
  if (-not $hasChanges -and -not $AllowEmptyCommit) {
    throw "No changes to commit. Use -AllowEmptyCommit to force a signed checkpoint commit."
  }

  git checkout -B $Branch

  if ($hasChanges) {
    git add .
    git commit -S -m $CommitMessage
  } else {
    git commit --allow-empty -S -m $CommitMessage
  }

  git push -u origin $Branch
}
finally {
  Pop-Location
}

$prUrl = gh pr create -R $Repository --base main --head $Branch --title $PrTitle --body $PrBody
Write-Log "Created PR: $prUrl"

try {
  if ($ToggleRuleset) {
    Invoke-RulesetToggle -State "disabled"
  }

  $mergeArgs = @("pr", "merge", $Branch, "-R", $Repository, "--$MergeMethod", "--delete-branch")
  if ($UseAdmin) {
    $mergeArgs += "--admin"
  }
  gh @mergeArgs
}
finally {
  if ($ToggleRuleset) {
    Invoke-RulesetToggle -State "active"
  }
}

$head = gh api "repos/$Repository/commits/main" | ConvertFrom-Json
$result = [ordered]@{
  repository = $Repository
  main_sha = $head.sha
  main_verified = $head.commit.verification.verified
  verification_reason = $head.commit.verification.reason
  message = $head.commit.message
  pr_url = $prUrl
}
$result | ConvertTo-Json -Depth 4
