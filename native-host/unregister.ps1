# SuperPasswdKey native messaging host 注销（SP-F-701）
# 等价于 register.ps1 -Unregister；独立提供便于一行操作：
#   powershell -ExecutionPolicy Bypass -File unregister.ps1 [-Browsers chrome,edge]

param(
    [string[]]$Browsers = @("chrome", "edge")
)

$script = Join-Path $PSScriptRoot "register.ps1"
& $script -Unregister -Browsers $Browsers
