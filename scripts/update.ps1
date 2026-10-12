[CmdletBinding()]
param([switch]$CheckOnly, [switch]$Local, [int]$WaitForProcessId = 0)
$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$installRoot = $PSScriptRoot
$statusPath = Join-Path $installRoot 'update-status.json'
$lock = $null
function Write-UpdateStatus($status) {
    $temporary = $statusPath + '.tmp'
    [System.IO.File]::WriteAllText($temporary, ($status | ConvertTo-Json -Depth 6), [System.Text.UTF8Encoding]::new($false))
    if (Test-Path -LiteralPath $statusPath) { [System.IO.File]::Replace($temporary, $statusPath, ($statusPath + '.previous')) } else { [System.IO.File]::Move($temporary, $statusPath) }
}
function Invoke-SourceGit([string[]]$GitArguments) {
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        $output = @(& git -C $script:sourceRoot @GitArguments 2>&1 | ForEach-Object { $_.ToString() })
        $gitExitCode = $LASTEXITCODE
    } finally { $ErrorActionPreference = $previousPreference }
    if ($gitExitCode -ne 0) { throw ('git ' + ($GitArguments -join ' ') + ' failed: ' + ($output -join [Environment]::NewLine)) }
    return $output -join [Environment]::NewLine
}
try {
    if ($CheckOnly -and $Local) { throw 'CheckOnly and Local are mutually exclusive' }
    if (!$CheckOnly) {
        $lock = [System.IO.File]::Open((Join-Path $installRoot 'update.lock'), [System.IO.FileMode]::OpenOrCreate, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
        Write-UpdateStatus @{ status = 'running'; mode = $(if ($Local) { 'local' } else { 'remote' }); started_at = [DateTime]::UtcNow.ToString('o'); updater_pid = $PID }
    }
    $record = Get-Content -LiteralPath (Join-Path $installRoot 'installation.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if (![string]::Equals([System.IO.Path]::GetFullPath($record.install_directory), [System.IO.Path]::GetFullPath($installRoot), [StringComparison]::OrdinalIgnoreCase)) { throw 'Installation directory does not match its record' }
    $script:sourceRoot = (Resolve-Path -LiteralPath $record.source_directory).Path
    if ((Invoke-SourceGit -GitArguments @('remote', 'get-url', 'origin')).Trim() -ne $record.remote) { throw 'Origin changed; refusing to update from a different repository' }
    if ((Invoke-SourceGit -GitArguments @('branch', '--show-current')).Trim() -ne $record.branch) { throw 'Source branch changed; switch back to the recorded branch first' }
    if (!$Local -and (Invoke-SourceGit -GitArguments @('status', '--porcelain')).Trim()) { throw 'Source has local changes; commit them first, or explicitly use update --local' }
    if ($WaitForProcessId -gt 0) {
        $deadline = [DateTime]::UtcNow.AddSeconds(30)
        while ($process = Get-Process -Id $WaitForProcessId -ErrorAction SilentlyContinue) {
            if (![string]::Equals($process.Path, (Join-Path $installRoot 'fantasy-lcsc.exe'), [StringComparison]::OrdinalIgnoreCase)) { throw 'Process to wait for is not this installed CLI' }
            if ([DateTime]::UtcNow -ge $deadline) { throw 'CLI did not exit; installed executable was not replaced' }
            Start-Sleep -Milliseconds 100
        }
    }
    if (!$Local) {
        # ls-remote only reads remote refs; checking does not write FETCH_HEAD.
        $remoteRef = 'refs/heads/' + $record.branch
        $remoteLine = (Invoke-SourceGit -GitArguments @('ls-remote', '--exit-code', 'origin', $remoteRef)).Trim()
        $remoteCommit = ($remoteLine -split '\s+')[0]
        $originalCommit = (Invoke-SourceGit -GitArguments @('rev-parse', 'HEAD')).Trim()
        if ($CheckOnly) {
            $ahead = $null; $behind = $null
            try {
                $counts = ((Invoke-SourceGit -GitArguments @('rev-list', '--left-right', '--count', ('HEAD...' + $remoteCommit))).Trim() -split '\s+')
                $ahead = [int]$counts[0]; $behind = [int]$counts[1]
            } catch { } # The remote commit may not yet exist in the local object store.
            $result = [ordered]@{ status = 'checked'; installed_commit = $record.source_commit; source_commit = $originalCommit; remote_commit = $remoteCommit; source_ahead = $ahead; source_behind = $behind; update_available = $record.source_commit -ne $remoteCommit; branch = $record.branch }
            $result | ConvertTo-Json -Depth 6
            exit 0
        }
        # Keep updater writes outside the user's working repository.
        $managedSource = Join-Path $installRoot 'source'
        if (!(Test-Path -LiteralPath $managedSource)) {
            $null = Invoke-SourceGit -GitArguments @('clone', '--branch', $record.branch, '--single-branch', $record.remote, $managedSource)
        }
        $script:sourceRoot = (Resolve-Path -LiteralPath $managedSource).Path
        if ((Invoke-SourceGit -GitArguments @('remote', 'get-url', 'origin')).Trim() -ne $record.remote -or (Invoke-SourceGit -GitArguments @('branch', '--show-current')).Trim() -ne $record.branch) { throw 'Managed source origin or branch changed' }
        if ((Invoke-SourceGit -GitArguments @('status', '--porcelain')).Trim()) { throw 'Managed source has local changes; refusing to overwrite them' }
        $null = Invoke-SourceGit -GitArguments @('fetch', '--no-tags', 'origin', $record.branch)
        $trackingRef = 'origin/' + $record.branch
        try { $null = Invoke-SourceGit -GitArguments @('merge-base', '--is-ancestor', $originalCommit, $trackingRef) }
        catch { throw 'Source has unpushed or diverged commits; refusing to replace local history' }
        $null = Invoke-SourceGit -GitArguments @('pull', '--ff-only', 'origin', $record.branch)
    }
    $installer = Join-Path $sourceRoot 'scripts\install.ps1'
    if (!(Test-Path -LiteralPath $installer -PathType Leaf)) { throw 'Source updater is missing install.ps1' }
    $parameters = @{ Destination = $installRoot; SourceDirectory = $sourceRoot; SkillDirectory = $record.skill_directory; AddToPath = [bool]$record.add_to_path }
    $installOutput = (& $installer @parameters | Out-String)
    Write-Output $installOutput.Trim()
    $installed = Get-Content -LiteralPath (Join-Path $installRoot 'installation.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    $result = @{ status = 'complete'; completed_at = [DateTime]::UtcNow.ToString('o'); source_commit = $installed.source_commit; cli_version = $installed.cli_version; executable_sha256 = $installed.executable_sha256 }
    Write-UpdateStatus $result
    $result | ConvertTo-Json -Depth 6
} catch {
    $result = @{ status = 'failed'; completed_at = [DateTime]::UtcNow.ToString('o'); error = $_.Exception.Message }
    # Only the lock owner publishes final update status; concurrent attempts cannot overwrite it.
    if (!$CheckOnly -and $null -ne $lock) {
        try { Write-UpdateStatus $result } catch { [Console]::Error.WriteLine('Unable to write update status: ' + $_.Exception.Message) }
    }
    [Console]::Error.WriteLine(($result | ConvertTo-Json -Compress))
    exit 1
} finally { if ($null -ne $lock) { $lock.Dispose() } }
