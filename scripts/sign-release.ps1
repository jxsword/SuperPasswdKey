# 发布签名工作流（DR-007）：本地对 CI 产物签名并发布 Release
#
# 前置（一次性，维护者离线执行，Agent 不经手口令）：
#   1. minisign -G                       # 强口令两次；默认生成 %USERPROFILE%\minisign.key + minisign.pub
#   2. 私钥 minisign.key 移至离线保管（加密 U 盘 + 打印备份）；本机不留副本
#   3. 将 minisign.pub 提交至仓库根 / GitHub Pages / Web Archive（四渠道，见 DR-007）
#
# 日常发布（本脚本，在装有 minisign 与 gh 的维护者机器上运行，签名口令交互输入）：
#   powershell -ExecutionPolicy Bypass -File scripts/sign-release.ps1 -Tag v0.2.0-ext `
#     [-SecretKey <离线私钥路径>]
#
# 产物：GitHub Release <Tag> 附 SHA256SUMS.txt / SHA256SUMS.txt.minisig / minisign.pub

param(
    [Parameter(Mandatory = $true)] [string]$Tag,
    [string]$SecretKey = "$env:USERPROFILE\.minisign\minisign.key",
    [string]$PubKey = "$env:USERPROFILE\.minisign\minisign.pub"
)

$ErrorActionPreference = "Stop"
$Repo = "jxsword/SuperPasswdKey"
$Work = Join-Path $env:TEMP "sp-release-$Tag"

if (-not (Get-Command minisign -ErrorAction SilentlyContinue)) { throw "minisign not found (winget install Minisign)" }
if (-not (Get-Command gh -ErrorAction SilentlyContinue))       { throw "gh not found" }
if (-not (Test-Path $SecretKey)) { throw "secret key not found: $SecretKey" }
if (-not (Test-Path $PubKey))    { throw "public key not found: $PubKey" }

# 1) 下载 CI 构建的发布 artifact
New-Item -ItemType Directory -Force -Path $Work | Out-Null
Push-Location $Work
gh release download $Tag --repo $Repo --pattern "sp-windows-release" --clobber
$zip = Get-ChildItem -File | Select-Object -First 1
if (-not $zip) { throw "no artifact downloaded for tag $Tag" }
Expand-Archive -Path $zip.Name -DestinationPath . -Force

# 2) 本地签名（minisign 交互式索要私钥口令）
$sums = "SHA256SUMS.txt"
if (-not (Test-Path $sums)) { throw "SHA256SUMS.txt not found in artifact" }
minisign -S -m $sums -s $SecretKey
if (-not (Test-Path "$sums.minisig")) { throw "signing produced no .minisig" }

# 3) 发布：Release 不存在则创建，随后上传产物与签名
$exists = gh release view $Tag --repo $Repo 2>$null
if ($LASTEXITCODE -ne 0) {
    gh release create $Tag --repo $Repo --title $Tag --generate-notes
}
gh release upload $Tag --repo $Repo $sums "$sums.minisig" $PubKey --clobber

Write-Host ""
Write-Host "Release $Tag signed and uploaded:"
Write-Host "  - $sums (from CI artifact)"
Write-Host "  - $sums.minisig (minisign, local key)"
Write-Host "  - minisign.pub (for verifiers)"
Pop-Location
Write-Host "Verify: minisign -V -p minisign.pub -m SHA256SUMS.txt"
