# SuperPasswdKey CLI 免安装包初始化（SP-C-104 分发线，v0.3.0 起）
# 用法（在解压目录内、PowerShell 执行）：
#   powershell -ExecutionPolicy Bypass -File install.ps1                       # 复制 + 加入用户 PATH
#   powershell -ExecutionPolicy Bypass -File install.ps1 -InstallDir D:\sp\bin # 自定义目录
#   powershell -ExecutionPolicy Bypass -File install.ps1 -AddPath:$false       # 不改 PATH
# 说明：仅写 HKCU（用户级，免管理员）；PATH 修改经 .NET API 自动广播生效；
#       浏览器填充的 NM host 注册见 native-host\register.ps1（需扩展 ID）。
param(
    [string]$InstallDir = (Join-Path $env:LOCALAPPDATA "SuperPasswdKey\bin"),
    [bool]$AddPath = $true
)
$ErrorActionPreference = "Stop"

$here = Split-Path -Parent $MyInvocation.MyCommand.Path
$spExe = Join-Path $here "sp.exe"
if (-not (Test-Path $spExe)) { throw "sp.exe not found next to install.ps1 ($here)" }

# 1) 复制可执行文件到安装目录
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
Copy-Item (Join-Path $here "sp.exe") $InstallDir -Force
if (Test-Path (Join-Path $here "sp-nmhost.exe")) {
    Copy-Item (Join-Path $here "sp-nmhost.exe") $InstallDir -Force
}
Write-Host "installed: $InstallDir"

# 2) 用户级 PATH（幂等追加；.NET SetEnvironmentVariable 自动广播 WM_SETTINGCHANGE）
#    守卫（2026-10-30 事件）：GetEnvironmentVariable 对超长值正常返回，但若返回空而
#    注册表实际存在非空 Path 值（异常状态）→ 立即中止，绝不覆盖。
if ($AddPath) {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    $envKey = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment")
    $rawExists = $envKey -and ($envKey.GetValueNames() -contains "Path")
    $rawValue = ""
    if ($rawExists) {
        $rawValue = $envKey.GetValue("Path", "", [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $envKey.Close()
    }
    if (-not $userPath -and $rawValue) {
        throw "用户 PATH 读取为空但注册表存在非空值（长度 $($rawValue.Length)）——为防覆盖已中止；请手动检查 HKCU\Environment\Path"
    }
    $parts = @()
    if ($userPath) { $parts = $userPath.Split(";") | Where-Object { $_ -ne "" } }
    if ($parts -notcontains $InstallDir) {
        $parts += $InstallDir
        [Environment]::SetEnvironmentVariable("Path", ($parts -join ";"), "User")
        Write-Host "PATH updated (user scope): $InstallDir"
        Write-Host "NOTE: 已开终端需重开才能拿到新 PATH。"
    } else {
        Write-Host "PATH already contains $InstallDir (skip)"
    }
}

# 3) 后续步骤提示
Write-Host ""
Write-Host "Next steps:"
Write-Host "  1. 重开一个终端，运行: sp init --db `"$env:APPDATA\SuperPasswdKey\vault.kdbx`""
Write-Host "     （生成 keyfile 后立即离线备份——keyfile 丢失库永久不可解）"
Write-Host "  2. 完整手册见随包 docs\manual\sp_cli_manual.md"
Write-Host "  3. 浏览器填充/自动保存：先构建/安装桌面壳或 sp-nmhost，"
Write-Host "     再运行 native-host\register.ps1（需要扩展 ID，见 native-host\README.md）"
