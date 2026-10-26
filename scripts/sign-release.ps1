# 发布签名工作流（DR-007）：本地对 CI 产物签名并发布 Release
#
# 前置（一次性，维护者离线执行，Agent 不经手口令）：
#   1. minisign -G                       # 强口令两次；生成 minisign.key + minisign.pub
#   2. 私钥 minisign.key 移至离线保管（加密 U 盘 + 打印备份）
#   3. 将 minisign.pub 提交至仓库根 / GitHub Pages / Web Archive（四渠道，见 DR-007）
#
# 日常发布（在维护者自己的 PowerShell 会话中运行，签名口令交互输入）：
#   & E:\proj\SuperPasswdKey\scripts\sign-release.ps1 -Tag v0.2.0-ext -ArtifactRunId <runId>
#
# 产物：GitHub Release <Tag> 附 zip / SHA256SUMS.txt / .minisig / minisign.pub

param(
    [Parameter(Mandatory = $true)] [string]$Tag,
    [string]$SecretKey = "$env:USERPROFILE\.minisign\minisign.key",
    [string]$PubKey = "$env:USERPROFILE\.minisign\minisign.pub",
    # Release 尚未创建时可从指定 CI run 下载 artifact
    [string]$ArtifactRunId = ""
)

# 原生命令用显式 $LASTEXITCODE 检查，而非 ErrorActionPreference=Stop
# （PS5.1 陷阱：gh 把进度/错误写到 stderr，EAP=Stop 会把正常流程当 terminating error 打死）
$ErrorActionPreference = "Continue"
$Repo = "jxsword/SuperPasswdKey"
$Work = Join-Path $env:TEMP "sp-release-$Tag"

function Invoke-Gh {
    gh @args
    if ($LASTEXITCODE -ne 0) {
        throw "gh failed (exit $LASTEXITCODE): gh $($args -join ' ')"
    }
}

function Test-ReleaseExists([string]$tag) {
    gh release view $tag --repo $Repo *> $null
    return ($LASTEXITCODE -eq 0)
}

if (-not (Get-Command minisign -ErrorAction SilentlyContinue)) { throw "minisign not found (winget install Minisign)" }
if (-not (Get-Command gh -ErrorAction SilentlyContinue))       { throw "gh not found" }
if (-not (Test-Path $SecretKey)) { throw "secret key not found: $SecretKey" }
if (-not (Test-Path $PubKey))    { throw "public key not found: $PubKey" }

# 1) 下载 CI 构建的发布产物（Release 已存在 → Release 资产；否则需 -ArtifactRunId）
New-Item -ItemType Directory -Force -Path $Work | Out-Null
Push-Location $Work
try {
    if (Test-ReleaseExists $Tag) {
        Invoke-Gh release download $Tag --repo $Repo --pattern "sp-windows-release" --clobber
    } elseif ($ArtifactRunId) {
        Invoke-Gh run download $ArtifactRunId --repo $Repo -n sp-windows-release -D $Work
    } else {
        throw "Release $Tag not found; pass -ArtifactRunId <run id> to fetch the CI artifact"
    }
    $zip = Get-ChildItem -File | Where-Object { $_.Name -match '\.zip$' } | Select-Object -First 1
    if (-not $zip) { throw "no artifact zip downloaded for tag $Tag" }
    Expand-Archive -Path $zip.Name -DestinationPath . -Force

    # 2) 本地签名（minisign 交互式索要私钥口令；失败由下方存在性检查兜底）
    $sums = "SHA256SUMS.txt"
    if (-not (Test-Path $sums)) { throw "SHA256SUMS.txt not found in artifact" }
    minisign -S -m $sums -s $SecretKey
    if (-not (Test-Path "$sums.minisig")) { throw "signing produced no .minisig (passphrase wrong or cancelled?)" }

    # 3) 发布：Release 不存在则创建，随后上传产物与签名
    if (-not (Test-ReleaseExists $Tag)) {
        Invoke-Gh release create $Tag --repo $Repo --title $Tag --target main --generate-notes
    }
    Invoke-Gh release upload $Tag --repo $Repo $zip.Name $sums "$sums.minisig" $PubKey --clobber

    Write-Host ""
    Write-Host "Release $Tag signed and uploaded:"
    Write-Host "  - $($zip.Name) (from CI artifact)"
    Write-Host "  - $sums (from CI artifact)"
    Write-Host "  - $sums.minisig (minisign, local key)"
    Write-Host "  - minisign.pub (for verifiers)"
    Write-Host "Verify: minisign -V -p minisign.pub -m SHA256SUMS.txt"
} finally {
    Pop-Location
}
