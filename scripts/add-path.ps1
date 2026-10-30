# Adds the install directory to the USER PATH safely (idempotent, type-preserving,
# long-value-safe). Used by sp-cli NSIS installer (v0.3.1+).
# Root-cause fix for the 2026-10-30 incident: NSIS ReadRegStr silently returns an
# EMPTY string for REG values longer than NSIS_MAX_STRLEN (1024) -- the old NSIS
# PATH logic then OVERWROTE the whole user PATH with just the install dir.
# This script reads via .NET RegistryKey (no length limit), appends idempotently,
# and writes back with the ORIGINAL value kind (REG_EXPAND_SZ preserved).
param(
    [Parameter(Mandatory = $true)][string]$InstallDir
)
$ErrorActionPreference = "Stop"

$key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey("Environment", $true)
if (-not $key) { throw "HKCU\Environment not found" }

$exists = $key.GetValueNames() -contains "Path"
if ($exists) {
    $kind = $key.GetValueKind("Path")
    $raw = $key.GetValue("Path", "", [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
} else {
    $kind = [Microsoft.Win32.RegistryValueKind]::ExpandString
    $raw = ""
}

# Hard guard: never overwrite with anything that does not contain the old value.
if ($raw -and -not $raw.EndsWith(";")) { $candidate = "$raw;$InstallDir" } else { $candidate = "$raw$InstallDir" }
if ($raw -and (-not $candidate.StartsWith($raw, [StringComparison]::OrdinalIgnoreCase))) {
    throw "PATH append sanity check failed -- aborting without writing (old value length $($raw.Length))"
}
$parts = $candidate -split ";" | Where-Object { $_ -ne "" }
$final = ($parts | Select-Object -Unique) -join ";"

$key.SetValue("Path", $final, $kind)
$key.Close()

# Broadcast so new shells pick it up without re-login
Add-Type -Namespace Win32 -Name NativeMethods -MemberDefinition @"
[System.Runtime.InteropServices.DllImport("user32.dll", SetLastError = true, CharSet = System.Runtime.InteropServices.CharSet.Auto)]
public static extern System.IntPtr SendMessageTimeout(System.IntPtr hWnd, uint Msg, System.UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out System.UIntPtr lpdwResult);
"@
$result = [System.UIntPtr]::Zero
[Win32.NativeMethods]::SendMessageTimeout([System.IntPtr]0xFFFF, 0x001A, [System.UIntPtr]::Zero, "Environment", 2, 5000, [ref]$result) | Out-Null

Write-Host "User PATH updated (kind=$kind, entries=$($final.Split(';').Count)); new terminals only."
