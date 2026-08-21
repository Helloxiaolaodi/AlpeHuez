#requires -Version 5.1
[CmdletBinding()]
param(
    [string]$Version = "",
    [switch]$NoBuild
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = Split-Path -Parent $PSScriptRoot
$releasesDir = Join-Path $root 'releases'
$tauriConfigPath = Join-Path $root 'src-tauri\tauri.conf.json'
$cargoTomlPath = Join-Path $root 'src-tauri\Cargo.toml'
$cargoLockPath = Join-Path $root 'src-tauri\Cargo.lock'
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)

function Get-AppVersion {
    $config = Get-Content -Raw -LiteralPath $tauriConfigPath | ConvertFrom-Json
    return $config.version
}

function Get-CargoVersion {
    $text = Get-Content -Raw -LiteralPath $cargoTomlPath
    if ($text -match '(?m)^version\s*=\s*"([^"]+)"') {
        return $Matches[1]
    }
    throw 'Cannot find version in Cargo.toml.'
}

function Get-CargoLockVersion {
    $text = Get-Content -Raw -LiteralPath $cargoLockPath
    $match = [regex]::Match(
        $text,
        '(?m)\[\[package\]\]\r?\nname = "my-nav-panel"\r?\nversion = "([^"]+)"'
    )
    if (-not $match.Success) {
        throw 'Cannot find the my-nav-panel package in Cargo.lock.'
    }
    return $match.Groups[1].Value
}

function Set-AppVersion {
    param([string]$NextVersion)

    $configText = Get-Content -Raw -LiteralPath $tauriConfigPath
    $configUpdated = $configText -replace
        '(?m)"version"\s*:\s*"[^"]+"',
        ('"version": "' + $NextVersion + '"')
    if ($configUpdated -eq $configText) {
        throw 'Could not update version in tauri.conf.json.'
    }
    [System.IO.File]::WriteAllText($tauriConfigPath, $configUpdated, $utf8NoBom)

    $cargoText = Get-Content -Raw -LiteralPath $cargoTomlPath
    $cargoUpdated = $cargoText -replace
        '(?m)^(version\s*=\s*")[^"]+(")',
        ('${1}' + $NextVersion + '${2}')
    if ($cargoUpdated -eq $cargoText) {
        throw 'Could not update version in Cargo.toml.'
    }
    [System.IO.File]::WriteAllText($cargoTomlPath, $cargoUpdated, $utf8NoBom)

    $lockText = Get-Content -Raw -LiteralPath $cargoLockPath
    $lockUpdated = $lockText -replace
        '(?m)(\[\[package\]\]\r?\nname = "my-nav-panel"\r?\nversion = ")[^"]+(")',
        ('${1}' + $NextVersion + '${2}')
    if ($lockUpdated -eq $lockText) {
        throw 'Could not update the my-nav-panel version in Cargo.lock.'
    }
    [System.IO.File]::WriteAllText($cargoLockPath, $lockUpdated, $utf8NoBom)

    Write-Host "Bumped AlpeHuez to v$NextVersion."
}

$versionToRelease = $Version.Trim()
if ($versionToRelease -match '^v(\d+\.\d+\.\d+)$') {
    $versionToRelease = $Matches[1]
}
if ([string]::IsNullOrWhiteSpace($versionToRelease)) {
    $versionToRelease = Get-AppVersion
}
if ($versionToRelease -notmatch '^\d+\.\d+\.\d+$') {
    throw 'Version must be in x.y.z or vx.y.z format.'
}

if ($PSBoundParameters.ContainsKey('Version')) {
    Set-AppVersion -NextVersion $versionToRelease
}

$configuredVersion = Get-AppVersion
if ($configuredVersion -ne $versionToRelease) {
    throw "tauri.conf.json is v$configuredVersion, expected v$versionToRelease."
}
$cargoVersion = Get-CargoVersion
if ($cargoVersion -ne $versionToRelease) {
    throw "Cargo.toml is v$cargoVersion, expected v$versionToRelease."
}
$lockVersion = Get-CargoLockVersion
if ($lockVersion -ne $versionToRelease) {
    throw "Cargo.lock is v$lockVersion, expected v$versionToRelease."
}

if (-not $NoBuild) {
    # 签名密钥：tauri-cli 只认 TAURI_SIGNING_PRIVATE_KEY（内容）。未设置时从 ~/.tauri/alpehuez.key 读取。
    if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
        $defaultKey = Join-Path $HOME '.tauri\alpehuez.key'
        if (Test-Path -LiteralPath $defaultKey) {
            $env:TAURI_SIGNING_PRIVATE_KEY = (Get-Content -Raw -LiteralPath $defaultKey).Trim()
            if (-not $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD) {
                Write-Warning 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD 未设置，签名会失败。'
            }
        }
        else {
            Write-Warning '未找到签名密钥（TAURI_SIGNING_PRIVATE_KEY 或 ~/.tauri/alpehuez.key），createUpdaterArtifacts 构建会失败。'
        }
    }
    # Resend API Key：构建期注入 option_env!("RESEND_API_KEY")。以 ~/.tauri/resend.key 为准（文件中的有效 key 会覆盖
    # 环境中可能残留的过期/无效 key——先前一次构建曾因 env 里的 401 key 编译进二进制而烧坏了找回密码功能）。
    $resendKey = Join-Path $HOME '.tauri\resend.key'
    if (Test-Path -LiteralPath $resendKey) {
        $env:RESEND_API_KEY = (Get-Content -Raw -LiteralPath $resendKey).Trim()
        Write-Host "Resend API Key loaded from $resendKey"
    }
    elseif (-not $env:RESEND_API_KEY) {
        Write-Warning '未找到 Resend API Key（~/.tauri/resend.key 或 RESEND_API_KEY），找回密码邮件发送会失败。'
    }
    Push-Location $root
    try {
        & tauri build --bundles nsis
        if ($LASTEXITCODE -ne 0) {
            throw "tauri build failed with exit code $LASTEXITCODE."
        }
    }
    finally {
        Pop-Location
    }
}

$releaseDir = Join-Path $releasesDir ("v" + $versionToRelease)
New-Item -ItemType Directory -Force -Path $releaseDir | Out-Null

$releaseExe = Join-Path $root 'src-tauri\target\release\my-nav-panel.exe'
if (Test-Path -LiteralPath $releaseExe) {
    Copy-Item -LiteralPath $releaseExe -Destination $releaseDir -Force
}

$nsisDir = Join-Path $root 'src-tauri\target\release\bundle\nsis'
$setupName = "AlpeHuez_${versionToRelease}_x64-setup.exe"
$setupPath = Join-Path $nsisDir $setupName
if (Test-Path -LiteralPath $setupPath) {
    Copy-Item -LiteralPath $setupPath -Destination $releaseDir -Force
}

$msiDir = Join-Path $root 'src-tauri\target\release\bundle\msi'
if (Test-Path -LiteralPath $msiDir) {
    Get-ChildItem -LiteralPath $msiDir -Filter ("*_" + $versionToRelease + "_x64_*.msi") |
        ForEach-Object {
            Copy-Item -LiteralPath $_.FullName -Destination $releaseDir -Force
        }
}

$hashLines = Get-ChildItem -LiteralPath $releaseDir -File |
    Where-Object { $_.Name -ne 'SHA256SUMS.txt' } |
    Sort-Object Name |
    ForEach-Object {
        $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash *$($_.Name)"
    }
$hashPath = Join-Path $releaseDir 'SHA256SUMS.txt'
[System.IO.File]::WriteAllLines($hashPath, $hashLines, [System.Text.Encoding]::ASCII)

$artifactNames = Get-ChildItem -LiteralPath $releaseDir -File |
    Where-Object { $_.Name -ne 'SHA256SUMS.txt' } |
    Select-Object -ExpandProperty Name
$manifest = [ordered]@{
    latest    = $versionToRelease
    updatedAt = (Get-Date).ToString('yyyy-MM-dd HH:mm:ss')
    artifacts = @($artifactNames)
}
$manifestText = $manifest | ConvertTo-Json
[System.IO.File]::WriteAllText(
    (Join-Path $releasesDir 'latest.json'),
    $manifestText,
    $utf8NoBom
)

# Tauri 官方 updater 清单（NSIS 安装版签名；便携 exe 无签名则跳过）
$sigPath = Join-Path $nsisDir ($setupName + '.sig')
if (Test-Path -LiteralPath $sigPath) {
    $signature = (Get-Content -Raw -LiteralPath $sigPath).Trim()
    # git 输出是 UTF-8，PowerShell 5.1 默认按系统代码页解码会乱码，先切到 UTF-8。
    $prevOut = [Console]::OutputEncoding
    [Console]::OutputEncoding = [System.Text.Encoding]::UTF8
    try {
        $notes = (git -C $root log -5 --oneline) -join "`n"
    }
    finally {
        [Console]::OutputEncoding = $prevOut
    }
    $pubDate = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    $updaterManifest = [ordered]@{
        version   = $versionToRelease
        notes     = $notes
        pub_date  = $pubDate
        platforms = [ordered]@{
            'windows-x86_64' = [ordered]@{
                signature = $signature
                url       = "https://raw.githubusercontent.com/Helloxiaolaodi/AlpeHuez/main/releases/v$versionToRelease/$setupName"
            }
        }
    }
    $updaterText = $updaterManifest | ConvertTo-Json -Depth 5
    [System.IO.File]::WriteAllText(
        (Join-Path $releasesDir 'updater-latest.json'),
        $updaterText,
        $utf8NoBom
    )
    Write-Host "Wrote updater-latest.json (signed installer)."
}
else {
    Write-Warning "No .sig at $sigPath — skipping updater-latest.json (portable-only release)."
}

Write-Host "Release v$versionToRelease archived at: $releaseDir"
