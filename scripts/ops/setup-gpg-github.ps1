param(
  [string]$RepositoryPath,
  [switch]$RequireGitHubKeyUpload
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Write-Log {
  param([string]$Message)
  $ts = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ss.fffZ")
  Write-Host "[$ts] $Message"
}

$gpg = "C:/Program Files/Git/usr/bin/gpg.exe"
if (-not (Test-Path $gpg)) {
  throw "GPG executable not found at $gpg"
}

$user = gh api user | ConvertFrom-Json
$login = $user.login
$uid = $user.id
$noreply = "$uid+$login@users.noreply.github.com"

Write-Log "Using GitHub noreply identity: $noreply"

& $gpg --list-secret-keys --keyid-format LONG $noreply | Out-Null
if ($LASTEXITCODE -ne 0) {
  Write-Log "No existing secret key for $noreply, generating"
  $batch = Join-Path $env:TEMP "astra-gh-gpg-batch.txt"
@"
%no-protection
Key-Type: RSA
Key-Length: 4096
Subkey-Type: RSA
Subkey-Length: 4096
Name-Real: $login
Name-Email: $noreply
Expire-Date: 0
%commit
"@ | Set-Content -Path $batch -NoNewline
  & $gpg --batch --generate-key $batch
}

$keyOut = & $gpg --list-secret-keys --keyid-format LONG $noreply
$keyLine = $keyOut | Select-String 'sec\s+\w+/([0-9A-F]+)'
if (-not $keyLine) {
  throw "Failed to parse GPG key id for $noreply"
}
$keyId = $keyLine.Matches[0].Groups[1].Value
Write-Log "Using key id: $keyId"

$pubKey = & $gpg --armor --export $keyId | Out-String
$tmpKey = Join-Path $env:TEMP "astra-gh-gpg-public.asc"
Set-Content -Path $tmpKey -Value $pubKey -NoNewline

$uploadSucceeded = $false
try {
  $keys = gh api user/gpg_keys | ConvertFrom-Json
  $exists = $false
  foreach ($k in $keys) {
    if ($k.public_key -eq $pubKey.Trim()) {
      $exists = $true
      break
    }
  }

  if (-not $exists) {
    gh api user/gpg_keys -X POST -f armored_public_key="@$tmpKey" | Out-Null
    Write-Log "Uploaded GPG public key to GitHub"
  } else {
    Write-Log "GPG public key already registered on GitHub"
  }
  $uploadSucceeded = $true
}
catch {
  Write-Log "Could not upload key to GitHub automatically: $($_.Exception.Message)"
  if ($RequireGitHubKeyUpload) {
    throw "GitHub GPG key upload is required but failed. Run: gh auth refresh -h github.com -s admin:gpg_key"
  }
}

if ($RepositoryPath) {
  $resolvedRepo = (Resolve-Path $RepositoryPath).Path
  Write-Log "Configuring Git signing in $resolvedRepo"
  Push-Location $resolvedRepo
  git config user.name $login
  git config user.email $noreply
  git config gpg.program $gpg
  git config commit.gpgsign true
  git config user.signingkey $keyId
  Pop-Location
}

$result = [ordered]@{
  login = $login
  noreply_email = $noreply
  signing_key_id = $keyId
  github_key_upload_succeeded = $uploadSucceeded
}

$result | ConvertTo-Json -Depth 4
