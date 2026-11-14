$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot
$engineRoot = Join-Path $projectRoot 'engine'
$sourceRoot = Join-Path $engineRoot 'uxplay'
$upstream = Get-Content (Join-Path $engineRoot 'upstream.json') -Raw | ConvertFrom-Json
$patch = Join-Path $engineRoot $upstream.patch
$sourceRevision = Join-Path $sourceRoot '.source-revision'
if ((Test-Path $sourceRevision) -and ((Get-Content $sourceRevision -Raw).Trim() -eq $upstream.revision)) { return }

if (-not (Test-Path (Join-Path $sourceRoot '.git'))) {
    git init $sourceRoot
    if ($LASTEXITCODE -ne 0) { throw 'Could not initialize the receiver source.' }
    git -C $sourceRoot remote add origin $upstream.repository
    git -C $sourceRoot fetch --depth 1 origin $upstream.revision
    if ($LASTEXITCODE -ne 0) { throw 'Could not fetch the pinned receiver revision.' }
    git -C $sourceRoot checkout --detach FETCH_HEAD
    if ($LASTEXITCODE -ne 0) { throw 'Could not check out the receiver source.' }
}

$revision = git -C $sourceRoot rev-parse HEAD
if ($revision -ne $upstream.revision) { throw 'The receiver checkout is not at the pinned revision.' }
$previousPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
git -C $sourceRoot apply --reverse --check $patch 2>$null
$alreadyApplied = $LASTEXITCODE -eq 0
$ErrorActionPreference = $previousPreference
if ($alreadyApplied) { return }
git -C $sourceRoot diff --quiet
if ($LASTEXITCODE -ne 0) { throw 'The receiver checkout has unrecognized changes.' }
git -C $sourceRoot apply --check $patch
if ($LASTEXITCODE -ne 0) { throw 'The receiver patch does not apply.' }
git -C $sourceRoot apply $patch
if ($LASTEXITCODE -ne 0) { throw 'Could not apply the receiver patch.' }
