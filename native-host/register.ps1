# SuperPasswdKey native messaging host 注册（SP-F-701，design_docs/11 §6 + design_docs/15）
# 用法：
#   powershell -ExecutionPolicy Bypass -File register.ps1 `
#     -HostExe "E:\proj\SuperPasswdKey\target\release\sp-nmhost.exe" `
#     -ExtensionId "abcdefghijklmnopabcdefghijklmnop" `
#     [-Browsers chrome,edge] [-SessionTimeout 300] [-Unregister]
#   # sp-desktop 注册模式（单进程托盘+NM host，DR-013；迁移期与 sp-nmhost 双轨，二选一注册）：
#   powershell -ExecutionPolicy Bypass -File register.ps1 `
#     -Target desktop `
#     -HostExe "E:\proj\SuperPasswdKey\desktop\target\release\sp-desktop.exe" `
#     -ExtensionId "abcdefghijklmnopabcdefghijklmnop" `
#     [-Browsers chrome,edge] [-Unregister]
# 说明：
#   - 仅写 HKCU（免管理员）；manifest 落位 %LOCALAPPDATA%\SuperPasswdKey\nm-host-manifest.json
#   - 同一 host 名（com.superpasswdkey.host）同一时刻只指向一个 exe：重跑脚本即切换
#     （迁移期双轨=可随时切回 -Target nmhost；extension 侧无感知）
#   - ExtensionId 为 unpacked 扩展 ID（chrome://extensions 页读取，32 位 a-p 字母）
#   - sp-nmhost 模式：测试库路径/keyfile 经 --db/--keyfile 或 config.toml 配置，不经 manifest
#   - sp-desktop 模式：库路径固定 %APPDATA%\SuperPasswdKey\vault.kdbx（桌面单库），
#     双超时经桌面设置窗（settings.json）；keyfile 可经 -KeyfilePath 传入（nm-host-config.json）

param(
    [Parameter(Mandatory = $false)] [string]$HostExe,
    [Parameter(Mandatory = $false)] [string]$ExtensionId,
    [ValidateSet("nmhost", "desktop")]
    [string]$Target = "nmhost",
    [string[]]$Browsers = @("chrome", "edge"),
    [int]$SessionTimeout = 300,
    [string]$DbPath = "",
    [string]$KeyfilePath = "",
    [switch]$Unregister
)

$ErrorActionPreference = "Stop"
$HostName = "com.superpasswdkey.host"
$ManifestDir = Join-Path $env:LOCALAPPDATA "SuperPasswdKey"
$ManifestPath = Join-Path $ManifestDir "nm-host-manifest.json"

$RegistryRoots = @{
    "chrome" = "HKCU:\Software\Google\Chrome\NativeMessagingHosts"
    "edge"   = "HKCU:\Software\Microsoft\Edge\NativeMessagingHosts"
}

if ($Unregister) {
    foreach ($b in $Browsers) {
        $key = Join-Path $RegistryRoots[$b] $HostName
        if (Test-Path $key) {
            Remove-Item -Path $key -Recurse
            Write-Host "unregistered: $b ($key)"
        } else {
            Write-Host "not registered: $key"
        }
    }
    if (Test-Path $ManifestPath) {
        Remove-Item $ManifestPath
        Write-Host "removed manifest: $ManifestPath"
    }
    exit 0
}

if (-not $HostExe)  { throw "-HostExe is required (path to sp-nmhost.exe or sp-desktop.exe)" }
if (-not $ExtensionId) { throw "-ExtensionId is required (32 chars, a-p)" }
if (-not (Test-Path $HostExe)) { throw "host binary not found: $HostExe" }
if ($ExtensionId -notmatch '^[a-p]{32}$') { throw "ExtensionId must be 32 chars of a-p: $ExtensionId" }
if ($Target -eq "nmhost" -and $HostExe -notmatch "sp-nmhost\.exe$") {
    Write-Warning "-Target nmhost 但 HostExe 不是 sp-nmhost.exe：$HostExe"
}
if ($Target -eq "desktop" -and $HostExe -notmatch "sp-desktop\.exe$") {
    Write-Warning "-Target desktop 但 HostExe 不是 sp-desktop.exe：$HostExe"
}

$resolvedHost = (Resolve-Path $HostExe).Path

$manifest = [ordered]@{
    name        = $HostName
    description = "SuperPasswdKey native messaging host (local password manager)"
    path        = $resolvedHost
    type        = "stdio"
    allowed_origins = @("chrome-extension://$ExtensionId/")
}
New-Item -ItemType Directory -Force -Path $ManifestDir | Out-Null
# 无 BOM UTF-8：Chrome manifest 解析与 Rust serde_json 均拒绝 BOM
$utf8NoBom = New-Object System.Text.UTF8Encoding($false)
[System.IO.File]::WriteAllText($ManifestPath, ($manifest | ConvertTo-Json), $utf8NoBom)
Write-Host "manifest written: $ManifestPath"

# host 侧 origin 绑定 + 库路径（第二道防线与库发现；Chrome manifest 不支持启动参数，
# host 自行读本文件——design_docs/11 §6）。desktop 模式不写 db_path（桌面壳固定
# %APPDATA% 单库）与 session_timeout_secs（双超时归桌面设置窗）。
$hostConfig = [ordered]@{
    allowed_origin       = "chrome-extension://$ExtensionId/"
}
if ($Target -eq "nmhost") {
    $hostConfig["session_timeout_secs"] = $SessionTimeout
    if ($DbPath)     { $hostConfig["db_path"] = (Resolve-Path $DbPath).Path }
}
if ($KeyfilePath) { $hostConfig["keyfile_path"] = (Resolve-Path $KeyfilePath).Path }
$hostConfigPath = Join-Path $ManifestDir "nm-host-config.json"
[System.IO.File]::WriteAllText($hostConfigPath, ($hostConfig | ConvertTo-Json), $utf8NoBom)
Write-Host "host config written: $hostConfigPath"

foreach ($b in $Browsers) {
    $dir = $RegistryRoots[$b]
    if (-not (Test-Path $dir)) {
        New-Item -Path $dir -ItemType Directory | Out-Null
    }
    $key = Join-Path $dir $HostName
    New-Item -Path $key -Value $ManifestPath -Force | Out-Null
    Write-Host "registered: $b -> $key"
}

Write-Host ""
if ($Target -eq "desktop") {
    Write-Host "Done (sp-desktop mode). Browser launches sp-desktop.exe per connectNative;"
    Write-Host "locked requests pop the desktop unlock window (DR-013 per-process first password)."
} else {
    Write-Host "Done. Host config (origin/session timeout/db path) written to nm-host-config.json;"
    Write-Host "sp-nmhost reads it at startup (design_docs/11 section 6)."
}
Write-Host "Verify in chrome://extensions that the extension ID matches -ExtensionId."
