# Permission guard for unattended agent runs in this repository.
# Reads the PermissionRequest event as JSON on stdin. Prints an "allow"
# decision only for shell commands and file edits that stay inside the repo
# and match none of the risky patterns below. Anything else prints nothing,
# so Kiro shows its normal approval prompt. It never denies on its own.

$ErrorActionPreference = 'Stop'
# The repository is where this script lives: <repo>\.kiro\hooks\scripts.
$Repo = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path

function Allow {
    '{"hookSpecificOutput":{"decision":{"behavior":"allow"}}}'
    exit 0
}
function Ask { exit 0 }

try {
    $raw = [Console]::In.ReadToEnd()
    $evt = $raw | ConvertFrom-Json
} catch { Ask }

$tool = [string]($evt.tool_name, $evt.toolName, $evt.tool | Where-Object { $_ } | Select-Object -First 1)
$input_ = $evt.tool_input
if (-not $input_) { $input_ = $evt.toolInput }
if (-not $input_) { $input_ = $evt.input }

function Inside([string]$path) {
    if (-not $path) { return $false }
    try {
        $full = [IO.Path]::GetFullPath($(if ([IO.Path]::IsPathRooted($path)) { $path } else { Join-Path $Repo $path }))
    } catch { return $false }
    return $full.ToLowerInvariant().StartsWith($Repo.ToLowerInvariant())
}

# File edits: inside the repo, never the .env file or the git internals.
if ($tool -match 'fs_write|str_replace|write|edit|replace') {
    $path = [string]($input_.path, $input_.file_path, $input_.filePath | Where-Object { $_ } | Select-Object -First 1)
    if ((Inside $path) -and ($path -notmatch '(^|[\\/])\.env($|\.)') -and ($path -notmatch '[\\/]\.git[\\/]')) { Allow }
    Ask
}

# Shell commands.
if ($tool -match 'pwsh|shell|bash|command|execute') {
    $cmd = [string]($input_.command, $input_.cmd | Where-Object { $_ } | Select-Object -First 1)
    $cwd = [string]($input_.cwd, $evt.cwd | Where-Object { $_ } | Select-Object -First 1)
    if (-not $cmd) { Ask }
    if ($cwd -and -not (Inside $cwd)) { Ask }

    $risky = @(
        'push\s+.*(--force|-f\b|--force-with-lease|--mirror|--delete|:refs)',
        'reset\s+--hard', 'clean\s+-[a-z]*f', 'filter-branch', 'rebase\b', 'branch\s+-D',
        'remote\s+(add|set-url|remove)', 'config\s+--global',
        'Set-ExecutionPolicy', 'shutdown', 'Stop-Computer', 'Restart-Computer', 'format\s+[a-z]:',
        '\breg(\.exe)?\s+(add|delete)', 'C:\\Windows', 'System32', 'Program Files[^\\]*\\(?!Microsoft\\Edge)',
        'Invoke-Expression', '\biex\b', 'DownloadString', 'Start-BitsTransfer',
        'npm\s+(publish|login|adduser)', 'cargo\s+(publish|login)', 'curl.+\|\s*(sh|bash|pwsh|powershell)'
    )
    foreach ($p in $risky) { if ($cmd -match $p) { Ask } }

    # git push only to origin main.
    if ($cmd -match '\bgit\s+push\b' -and $cmd -notmatch '\bgit\s+push\s+origin\s+main\b') { Ask }

    # .env: only loaded into the process environment, never printed or edited.
    if ($cmd -match '\.env\b' -and $cmd -notmatch '\.gitignore') {
        $loadsOnly = $cmd -match "Get-Content\s+\.env\s*\|\s*ForEach-Object\s*\{\s*if\s*\(\s*\`$_\s+-match" -and $cmd -match 'SetEnvironmentVariable'
        if (-not $loadsOnly) { Ask }
        if ($cmd -match 'Write-(Host|Output)|echo|Out-File|Set-Content|Add-Content|\$env:NVIDIA_API_KEY\b(?!\s*=)') { Ask }
    }

    # Deletions: only relative paths or paths inside the repo.
    if ($cmd -match '\b(Remove-Item|rm|rmdir|del|rd)\b') {
        if ($cmd -match '(~|\$env:(USERPROFILE|HOME|APPDATA|LOCALAPPDATA)|\$HOME)') {
            if ($cmd -notmatch '\$env:TEMP') { Ask }
        }
        foreach ($m in [regex]::Matches($cmd, '[A-Za-z]:\\[^\s"''|;]*')) {
            if (-not (Inside $m.Value)) { Ask }
        }
    }

    # Absolute paths written to: inside the repo or the temp folder only.
    foreach ($m in [regex]::Matches($cmd, '(Set-Content|Add-Content|Out-File|New-Item|Copy-Item|Move-Item|WriteAllText)[^|;]*?([A-Za-z]:\\[^\s"''|;)]*)')) {
        $target = $m.Groups[2].Value
        if (-not (Inside $target) -and $target -notmatch '\\AppData\\Local\\Temp\\') { Ask }
    }

    Allow
}

Ask
