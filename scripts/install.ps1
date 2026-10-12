[CmdletBinding()]
param(
    [string]$Destination = 'D:\Agent\LocalManager\FantasyLCSC',
    [string]$SourceDirectory = (Split-Path -Parent $PSScriptRoot),
    [string]$SkillDirectory = (Join-Path $env:USERPROFILE '.codex\skills'),
    [switch]$SkipBuild,
    [switch]$AddToPath
)
$ErrorActionPreference = 'Stop'
function Get-ExecutableSha256([string]$Path) {
    $algorithm = [System.Security.Cryptography.SHA256]::Create()
    try { return [BitConverter]::ToString($algorithm.ComputeHash([System.IO.File]::ReadAllBytes($Path))).Replace('-', '') }
    finally { $algorithm.Dispose() }
}
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$sourceRoot = (Resolve-Path -LiteralPath $SourceDirectory).Path
$installRoot = [System.IO.Path]::GetFullPath($Destination)
$skillRoot = [System.IO.Path]::GetFullPath($SkillDirectory)
if ($installRoot.TrimEnd('\') -eq [System.IO.Path]::GetPathRoot($installRoot).TrimEnd('\')) { throw 'Cannot install into a drive root' }
$sourceSkill = Join-Path $sourceRoot 'skills\easyeda-engineering'
if (!(Test-Path -LiteralPath (Join-Path $sourceRoot 'Cargo.toml')) -or !(Test-Path -LiteralPath (Join-Path $sourceSkill 'SKILL.md'))) { throw 'Source repository is incomplete' }
if (!$SkipBuild) {
    Push-Location $sourceRoot
    try {
        & cargo build --release --locked
        if ($LASTEXITCODE -ne 0) { throw 'Rust build failed; installed files were not changed' }
    } finally { Pop-Location }
}
$candidate = Join-Path $sourceRoot 'target\release\fantasy-lcsc.exe'
if (!(Test-Path -LiteralPath $candidate -PathType Leaf)) { throw 'CLI executable is missing; build first' }
$version = (& $candidate --version | Out-String).Trim()
if ($LASTEXITCODE -ne 0 -or $version -notmatch '^fantasy-lcsc \d+\.\d+\.\d+') { throw 'Candidate executable failed version check' }
$sourceCommit = (& git -C $sourceRoot rev-parse HEAD | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify source Git commit' }
$sourceRemote = (& git -C $sourceRoot remote get-url origin | Out-String).Trim()
if ($LASTEXITCODE -ne 0) { throw 'Cannot identify origin' }
$sourceBranch = (& git -C $sourceRoot branch --show-current | Out-String).Trim()
if (!$sourceBranch) { throw 'Installation needs a named source branch' }
$sourceDirty = @(& git -C $sourceRoot status --porcelain).Count -gt 0

New-Item -ItemType Directory -Force -Path $installRoot, $skillRoot | Out-Null
$backupRoot = Join-Path $installRoot ('skill-backups\' + (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [guid]::NewGuid().ToString('N'))
$stagingRoot = Join-Path $installRoot ('.skill-stage-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Force -Path $stagingRoot | Out-Null
Copy-Item -LiteralPath $sourceSkill -Destination $stagingRoot -Recurse
$stagedSkill = Join-Path $stagingRoot 'easyeda-engineering'
$installedSkill = Join-Path $skillRoot 'easyeda-engineering'
$movedSkills = @()
try {
    # Verify exact resolved roots before directory moves; preserve old skills in backups.
    foreach ($name in @('easyeda-engineering', 'easyeda-agent-access', 'personal-eda-style')) {
        $oldSkill = Join-Path $skillRoot $name
        if (Test-Path -LiteralPath $oldSkill -PathType Container) {
            $resolvedOld = (Resolve-Path -LiteralPath $oldSkill).Path
            if (![string]::Equals($resolvedOld, [System.IO.Path]::GetFullPath($oldSkill), [StringComparison]::OrdinalIgnoreCase) -or !$resolvedOld.StartsWith($skillRoot.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Skill path escapes intended root' }
            New-Item -ItemType Directory -Force -Path $backupRoot | Out-Null
            $backup = Join-Path $backupRoot $name
            if (!$backup.StartsWith($installRoot.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Backup path escapes installation root' }
            Move-Item -LiteralPath $resolvedOld -Destination $backup
            $movedSkills += @{ original = $oldSkill; backup = $backup }
        }
    }
    if (!(Resolve-Path -LiteralPath $stagedSkill).Path.StartsWith($installRoot.TrimEnd('\') + '\', [StringComparison]::OrdinalIgnoreCase)) { throw 'Staged skill escapes installation root' }
    Move-Item -LiteralPath $stagedSkill -Destination $installedSkill
} catch {
    foreach ($entry in $movedSkills) {
        if (!(Test-Path -LiteralPath $entry.original) -and (Test-Path -LiteralPath $entry.backup)) { Move-Item -LiteralPath $entry.backup -Destination $entry.original }
    }
    throw
}
# The staging parent is empty after publishing the skill; do not recursively delete it.
Remove-Item -LiteralPath $stagingRoot

$installedExe = Join-Path $installRoot 'fantasy-lcsc.exe'
$temporaryExe = Join-Path $installRoot ('.fantasy-lcsc-' + [guid]::NewGuid().ToString('N') + '.exe')
try {
    Copy-Item -LiteralPath $candidate -Destination $temporaryExe
    if (Test-Path -LiteralPath $installedExe) {
        [System.IO.File]::Replace($temporaryExe, $installedExe, (Join-Path $installRoot 'fantasy-lcsc.previous.exe'))
    } else { [System.IO.File]::Move($temporaryExe, $installedExe) }
} finally { if (Test-Path -LiteralPath $temporaryExe) { Remove-Item -LiteralPath $temporaryExe } }
Copy-Item -LiteralPath (Join-Path $sourceRoot 'scripts\update.ps1') -Destination (Join-Path $installRoot 'update.ps1')
Copy-Item -LiteralPath (Join-Path $sourceRoot 'README.md'), (Join-Path $sourceRoot 'LICENSE') -Destination $installRoot
$docsRoot = Join-Path $installRoot 'docs'
New-Item -ItemType Directory -Force -Path $docsRoot | Out-Null
Get-ChildItem -LiteralPath (Join-Path $sourceRoot 'docs') -Filter '*.md' -File | Copy-Item -Destination $docsRoot
if ($AddToPath) {
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $pathEntries = @($userPath -split ';' | Where-Object { $_ })
    if (!($pathEntries | Where-Object { $_.TrimEnd('\') -ieq $installRoot.TrimEnd('\') })) {
        [Environment]::SetEnvironmentVariable('Path', (($pathEntries + $installRoot) -join ';'), 'User')
    }
    if (!($env:Path -split ';' | Where-Object { $_.TrimEnd('\') -ieq $installRoot.TrimEnd('\') })) { $env:Path += ';' + $installRoot }
}
$record = [ordered]@{
    schema_version = 1; cli_version = $version; installed_at = [DateTime]::UtcNow.ToString('o')
    install_directory = $installRoot; source_directory = $sourceRoot; source_commit = $sourceCommit
    source_dirty = $sourceDirty; remote = $sourceRemote; branch = $sourceBranch
    skill_directory = $skillRoot; skill_name = 'easyeda-engineering'; add_to_path = [bool]$AddToPath
    executable_sha256 = Get-ExecutableSha256 $installedExe
    skill_backups = @($movedSkills | ForEach-Object { $_.backup })
}
$manifest = Join-Path $installRoot 'installation.json'
$manifestTemp = $manifest + '.tmp'
[System.IO.File]::WriteAllText($manifestTemp, ($record | ConvertTo-Json -Depth 6), [System.Text.UTF8Encoding]::new($false))
if (Test-Path -LiteralPath $manifest) { [System.IO.File]::Replace($manifestTemp, $manifest, ($manifest + '.previous')) } else { [System.IO.File]::Move($manifestTemp, $manifest) }
$record | ConvertTo-Json -Depth 6
