# Publish VoxelSeek to your GitHub account (PowerShell)
#
# Usage:
#   1. Create an EMPTY repository on GitHub (no README/license), e.g. "voxelseek".
#   2. Edit $GitHubUser below or pass -GitHubUser <name>.
#   3. Run:  powershell -ExecutionPolicy Bypass -File scripts/publish.ps1 -GitHubUser <name>
#
# The script uses an existing git credential helper (e.g. Git Credential
# Manager) for authentication. It never stores or prints your token.

param(
    [Parameter(Mandatory = $true)]
    [string]$GitHubUser,
    [string]$RepoName = "voxelseek"
)

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot

Write-Host "==> Checking git identity"
if (-not (git config user.name)) {
    git config user.name $GitHubUser
}
if (-not (git config user.email)) {
    git config user.email "$GitHubUser@users.noreply.github.com"
}

Write-Host "==> Committing pending changes (if any)"
git add -A
$pending = git status --porcelain
if ($pending) {
    git commit -m "chore: update before publishing"
}

$remote = "https://github.com/$GitHubUser/$RepoName.git"
Write-Host "==> Setting remote 'origin' to $remote"
if (git remote get-url origin 2>$null) {
    git remote set-url origin $remote
} else {
    git remote add origin $remote
}

Write-Host "==> Pushing main (you may be asked to sign in once)"
git push -u origin main

Write-Host "==> Done: https://github.com/$GitHubUser/$RepoName"
