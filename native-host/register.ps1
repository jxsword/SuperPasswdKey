# SuperPasswdKey native messaging host 注册（SP-F-701，design_docs/11 §6）
# 用法：
#   powershell -ExecutionPolicy Bypass -File register.ps1 `
#     -HostExe "E:\proj\SuperPasswdKey\target\release\sp-nmhost.exe" `
#     -ExtensionId "abcdefghijklmnopabcdefghijklmnop" `
#     [-Browsers chrome,edge] [-SessionTimeout 300] [-Unregister]
# 说明：
#   - 仅写 HKCU（免管理员）；manifest 落位 %LOCALAPPDATA%\SuperPasswdKey\nm-host-manifest.json
#   - ExtensionId 为 unpacked 扩展 ID（chrome://extensions 页读取，32 位 a-p 字母）
#   - 测试库路径/keyfile 经 sp-nmhost --db/--keyfile 或 config.toml 配置，不经 manifest

param(
    [Parameter(Mandatory = $false)] [string]$HostExe,
    [Parameter(Mandatory = $false)] [string]$ExtensionId,
    [string[]]$Browsers = @("chrome", "edge"),
    [int]$SessionTimeout = 300,
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

if (-not $HostExe)  { throw "-HostExe is required (path to sp-nmhost.exe)" }
if (-not $ExtensionId) { throw "-ExtensionId is required (32 chars, a-p)" }
if (-not (Test-Path $HostExe)) { throw "host binary not found: $HostExe" }
if ($ExtensionId -notmatch '^[a-p]{32}$') { throw "ExtensionId must be 32 chars of a-p: $ExtensionId" }

$resolvedHost = (Resolve-Path $HostExe).Path

$manifest = [ordered]@{
    name        = $HostName
    description = "SuperPasswdKey native messaging host (local password manager)"
    path        = $resolvedHost
    type        = "stdio"
    allowed_origins = @("chrome-extension://$ExtensionId/")
}
New-Item -ItemType Directory -Force -Path $ManifestDir | Out-Null
$manifest | ConvertTo-Json | Set-Content -Path $ManifestPath -Encoding UTF8
Write-Host "manifest written: $ManifestPath"

# host 侧 origin 绑定（第二道防线；Chrome manifest 不支持启动参数，
# host 自行读本文件——design_docs/11 §6）
$hostConfigPath = Join-Path $ManifestDir "nm-host-config.json"
[ordered]@{
    allowed_origin    = "chrome-extension://$ExtensionId/"
    session_timeout_secs = $SessionTimeout
} | ConvertTo-Json | Set-Content -Path $hostConfigPath -Encoding UTF8
Write-Host "host config written: $hostConfigPath"

foreach ($b in $Browsers) {
    $dir = $RegistryRoots[$b]
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $key = Join-Path $dir $HostName
    New-Item -ItemType String -Path $key -Value $ManifestPath -Force | Out-Null
    Write-Host "registered: $b -> $key"
}

Write-Host ""
Write-Host "host launch args are appended by design (session timeout ${SessionTimeout}s):"
Write-Host "  sp-nmhost --allowed-origin chrome-extension://$ExtensionId/ --session-timeout $SessionTimeout"
Write-Host "NOTE: this script writes the manifest 'path' only; pass --session-timeout via config if non-default."
Write-Host "Verify in chrome://extensions that the extension ID matches -ExtensionId."
