param(
    # dev.twitch.tv で登録した Client ID。省略すると src-tauri/src/twitch.rs の BUILTIN_CLIENT_ID を使う
    [string]$ClientId = $env:STREAMSTATUS_CLIENT_ID
)

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

$projectRoot = Split-Path -Parent $PSScriptRoot
$tauriDir = Join-Path $projectRoot 'src-tauri'
$artifacts = Join-Path $projectRoot 'artifacts'

# ビルドの中間ファイルは数 GB になるので、Google ドライブで同期しない場所に置く
$env:CARGO_TARGET_DIR = Join-Path $env:LOCALAPPDATA 'cargo-target\stream-status'
if ($ClientId) {
    $env:STREAMSTATUS_CLIENT_ID = $ClientId
}

$version = (Get-Content -LiteralPath (Join-Path $tauriDir 'tauri.conf.json') -Raw | ConvertFrom-Json).version
if (-not $version) {
    throw 'tauri.conf.json から version を読めませんでした'
}

Push-Location $tauriDir
try {
    & cargo tauri build
    if ($LASTEXITCODE -ne 0) {
        throw "cargo tauri build が失敗しました (終了コード $LASTEXITCODE)"
    }
}
finally {
    Pop-Location
}

$exe = Join-Path $env:CARGO_TARGET_DIR 'release\StreamStatus.exe'
if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) {
    throw "StreamStatus.exe が見つかりません: $exe"
}

New-Item -ItemType Directory -Force $artifacts | Out-Null
$releaseExe = Join-Path $artifacts "StreamStatus-v$version.exe"
Copy-Item -LiteralPath $exe -Destination $releaseExe -Force

$sizeMiB = [Math]::Round((Get-Item -LiteralPath $releaseExe).Length / 1MB, 2)
Write-Host "配布用: $releaseExe ($sizeMiB MiB)"
$builtinEmpty = Select-String -LiteralPath (Join-Path $tauriDir 'src\twitch.rs') -Pattern 'BUILTIN_CLIENT_ID: &str = "";' -Quiet
if (-not $ClientId -and $builtinEmpty) {
    Write-Host '注意: Client ID が入っていません。ログインできない exe になります'
}
