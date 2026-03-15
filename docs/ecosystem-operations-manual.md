# Ecosystem Operations Manual

This manual defines the fully automated, auditable operations flow for the Astra ecosystem:

- `Astra` (CLI + automation scripts)
- `packages` (recipes + package build workflow)
- `altair-repo` (artifacts + index)

All operations are fail-fast and restore branch protection immediately after any temporary disable.

## 1) Script Inventory

Location: `Astra/scripts/ops`

- `lifecycle-e2e.ps1`
  - Builds core packages (`nano`, `curl`, `htop`, `jq`, `ripgrep`, `tree`)
  - Verifies artifact existence + checksums
  - Generates and validates `index.json`
  - Starts local repo server and runs install/remove/upgrade lifecycle
  - Writes timestamped logs + JSON report

- `toggle-ruleset.ps1`
  - Safely sets ruleset enforcement (`active` / `disabled`)
  - Verifies enforcement state after update

- `setup-gpg-github.ps1`
  - Generates local GPG key if missing
  - Configures git signing identity
  - Attempts GitHub key upload (supports strict mode with `-RequireGitHubKeyUpload`)

- `pr-merge-safe.ps1`
  - Creates branch, signed commit, PR
  - Optionally toggles rulesets for protected merges
  - Squash/rebase merge via `gh`
  - Verifies `main` commit verification metadata

- `dashboard.ps1`
  - Displays latest commit status per repo
  - Shows verification state, open PR count, workflow count
  - Can include lifecycle report summary

- `final-verify.ps1`
  - Validates lifecycle report success
  - Verifies all repo heads are signed/verified and no open PRs
  - Verifies `altair-repo` index/package checksum integrity

## 2) Prerequisites

- `gh` authenticated with repo admin/push rights
- Rust toolchain available for `astra` build
- PowerShell 7+ recommended
- Optional GitHub GPG key upload scope for automation:
  - `admin:gpg_key`

## 3) Lifecycle Automation Usage

```powershell
./Astra/scripts/ops/lifecycle-e2e.ps1 `
  -WorkspaceRoot "C:/Users/Aaryadev/Desktop/Atlar/workspace" `
  -RunId "manual-$(Get-Date -Format yyyyMMddHHmmss)"
```

Outputs:

- Runtime logs: `.ops-runtime/<run-id>/logs/*`
- Artifact + lifecycle report: `.ops-runtime/<run-id>/lifecycle-report.json`

## 4) Branch Protection Toggle Workflow

Disable before protected emergency merge:

```powershell
./Astra/scripts/ops/toggle-ruleset.ps1 -Repository "Altair-Linux/packages" -Enforcement disabled
```

Re-enable immediately after merge/push:

```powershell
./Astra/scripts/ops/toggle-ruleset.ps1 -Repository "Altair-Linux/packages" -Enforcement active
```

## 5) Signed Commit + PR Merge Automation

```powershell
./Astra/scripts/ops/pr-merge-safe.ps1 `
  -Repository "Altair-Linux/packages" `
  -RepositoryPath "C:/Users/Aaryadev/Desktop/Atlar/workspace/packages" `
  -Branch "finalize-ecosystem" `
  -CommitMessage "chore: automated checkpoint" `
  -PrTitle "Finalize ecosystem" `
  -PrBody "Automated signed PR flow" `
  -MergeMethod squash `
  -UseAdmin `
  -ToggleRuleset
```

## 6) CI/CD Triggers and Logs

### Astra workflow

File: `.github/workflows/ecosystem-automation.yml`

Triggers:

- PR to `main`
- Push to `main`
- `repository_dispatch` with `packages-updated`
- Manual dispatch

Behavior:

- Runs lifecycle automation + final verification
- Uploads operation logs as artifacts
- Sends optional Discord notification on failure

### Packages workflow

File: `packages/.github/workflows/package-build.yml`

Behavior highlights:

- Auto-discovery package matrix
- Sequential package builds + lifecycle smoke test
- Standardized artifact output
- Automated altair-repo PR sync job
- Triggers Astra workflow via `repository_dispatch`

## 7) Monitoring Dashboard

```powershell
./Astra/scripts/ops/dashboard.ps1 `
  -LifecycleReportPath "C:/Users/Aaryadev/Desktop/Atlar/workspace/.ops-runtime/manual/lifecycle-report.json"
```

Displays:

- Latest commit SHA/message per repo
- `verified` + verification reason
- Open PR count
- Workflow counts
- Lifecycle artifacts summary

## 8) Final Closure Verification

```powershell
./Astra/scripts/ops/final-verify.ps1 `
  -WorkspaceRoot "C:/Users/Aaryadev/Desktop/Atlar/workspace" `
  -RunLifecycle `
  -LifecycleRunId "closure"
```

Pass criteria:

- Lifecycle report status is `success`
- All three repos have `main` heads with `verified: true`
- No open PRs remain
- `altair-repo/unstable/index.json` matches actual package files + checksums

## 9) Safe Remediation for Unverified Heads

1. Prefer PR-based GitHub squash/rebase merge to produce verified GitHub merge commit.
2. If blocked by rules requiring unresolved conversations/reviews and you are sole maintainer:
   - Disable ruleset enforcement temporarily.
   - Merge with approved strategy.
   - Re-enable ruleset enforcement immediately.
3. Re-run `final-verify.ps1` and confirm all heads are verified.

## 10) Audit Trail Checklist

For each automation run, retain:

- Workflow run URL
- Lifecycle transcript + server logs
- Lifecycle JSON report
- PR URLs and merge timestamps
- Ruleset toggle timestamps (disable/enable)
- Final verification JSON output
