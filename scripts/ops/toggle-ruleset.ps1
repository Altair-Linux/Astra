param(
  [Parameter(Mandatory = $true)]
  [string]$Repository,
  [string]$RulesetName = "main-protection",
  [Parameter(Mandatory = $true)]
  [ValidateSet("active", "disabled")]
  [string]$Enforcement
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Log {
  param([string]$Message)
  $ts = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
  Write-Host "[$ts] $Message"
}

Write-Log "Fetching rulesets for $Repository"
$rulesets = gh api "repos/$Repository/rulesets" | ConvertFrom-Json
$ruleset = $rulesets | Where-Object { $_.name -eq $RulesetName } | Select-Object -First 1
if (-not $ruleset) {
  throw "Ruleset '$RulesetName' not found in $Repository"
}

$full = gh api "repos/$Repository/rulesets/$($ruleset.id)" | ConvertFrom-Json
$payload = [ordered]@{
  name = $full.name
  target = $full.target
  enforcement = $Enforcement
  conditions = $full.conditions
  rules = $full.rules
  bypass_actors = @()
} | ConvertTo-Json -Depth 40

$tmp = Join-Path $env:TEMP ("{0}-{1}-ruleset.json" -f ($Repository -replace '/','-'), $Enforcement)
Set-Content -Path $tmp -Value $payload -NoNewline

Write-Log "Applying enforcement='$Enforcement' to ruleset '$RulesetName'"
$updated = gh api -X PUT "repos/$Repository/rulesets/$($ruleset.id)" --input $tmp | ConvertFrom-Json
if ($updated.enforcement -ne $Enforcement) {
  throw "Ruleset update failed. Expected '$Enforcement', got '$($updated.enforcement)'"
}

Write-Log "Ruleset '$RulesetName' enforcement is now '$($updated.enforcement)'"
