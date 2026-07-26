# WinSLA Credential Provider Diagnostic Script
# Run as Administrator on the target machine AFTER installation.
# Purpose: determine whether the dual-auth tile failure is in the DLL/COM layer
#          or in the LogonUI policy/enumeration layer, by loading the DLL and
#          instantiating the COM object OUTSIDE of LogonUI.
#
# Output: screen + C:\Temp\winsla-diag.txt  (send this file back for analysis)

$ErrorActionPreference = "Continue"
$clsid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"
$report = "C:\Temp\winsla-diag.txt"
New-Item -ItemType Directory -Path "C:\Temp" -Force | Out-Null
"" | Out-File $report -Encoding utf8

function Log($msg) {
    Write-Host $msg
    Add-Content -Path $report -Value $msg
}

Log "=================================================="
Log "WinSLA CP Diagnostic - $(Get-Date)"
Log "OS: $([System.Environment]::OSVersion.VersionString)"
Log "=================================================="

# --- 1. Registry (64-bit view) -----------------------------------
Log ""
Log "[1] Registry (64-bit view)"
$cpPath = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"
$clsidPath = "HKLM:\SOFTWARE\Classes\CLSID\$clsid"
$inprocPath = "$clsidPath\InprocServer32"

if (Test-Path $cpPath) {
    $cp = Get-ItemProperty $cpPath
    Log "  CP key exists. Disabled=$($cp.Disabled)  (Default)='$($cp.'(default)')'"
    Log "  DllPath=$($cp.DllPath)"
} else {
    Log "  ERROR: CP key MISSING: $cpPath"
}
if (Test-Path $inprocPath) {
    $ip = Get-ItemProperty $inprocPath
    Log "  InprocServer32 (Default)='$($ip.'(default)')'"
    Log "  ThreadingModel='$($ip.ThreadingModel)'"
    $dllPath = $ip.'(default)'
} else {
    Log "  ERROR: InprocServer32 MISSING: $inprocPath"
    $dllPath = "C:\Program Files\WinSLA\DualAuthCP.dll"
}

# --- 2. DLL file + PE architecture -------------------------------
Log ""
Log "[2] DLL file check"
if (Test-Path $dllPath) {
    $fi = Get-Item $dllPath
    Log "  DLL exists: $dllPath ($($fi.Length) bytes, $($fi.LastWriteTime))"
    try {
        $bytes = [System.IO.File]::ReadAllBytes($dllPath)
        $peOff = [BitConverter]::ToInt32($bytes, 0x3C)
        $machine = [BitConverter]::ToUInt16($bytes, $peOff + 4)
        $arch = if ($machine -eq 0x8664) { "x64 (OK)" } elseif ($machine -eq 0x014c) { "x86 (WRONG for 64-bit LogonUI!)" } else { "unknown 0x$($machine.ToString('X4'))" }
        Log "  PE machine = 0x$($machine.ToString('X4')) -> $arch"
    } catch {
        Log "  Could not parse PE header: $_"
    }
} else {
    Log "  ERROR: DLL NOT FOUND at $dllPath"
}

# --- 3. LoadLibrary + DllGetClassObject + CreateInstance ---------
Log ""
Log "[3] LoadLibrary / COM instantiation test (outside LogonUI)"
$source = @'
using System;
using System.Runtime.InteropServices;

[StructLayout(LayoutKind.Sequential)]
public struct GUID {
    public uint Data1; public ushort Data2; public ushort Data3;
    [MarshalAs(UnmanagedType.ByValArray, SizeConst = 8)] public byte[] Data4;
    public GUID(string g) {
        var x = new Guid(g); var b = x.ToByteArray();
        Data1 = (uint)(b[0] | (b[1]<<8) | (b[2]<<16) | (b[3]<<24));
        Data2 = (ushort)(b[4] | (b[5]<<8));
        Data3 = (ushort)(b[6] | (b[7]<<8));
        Data4 = new byte[]{ b[8],b[9],b[10],b[11],b[12],b[13],b[14],b[15] };
    }
}

public static class CPDiag {
    [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Unicode)]
    public static extern IntPtr LoadLibraryW(string lpFileName);
    [DllImport("kernel32.dll", SetLastError=true, CharSet=CharSet.Ansi)]
    public static extern IntPtr GetProcAddress(IntPtr hModule, string lpProcName);
    [DllImport("kernel32.dll")]
    public static extern bool FreeLibrary(IntPtr hModule);
    // Standard COM activation - the same mechanism LogonUI uses.
    [DllImport("ole32.dll")]
    public static extern int CoCreateInstance(ref Guid rclsid, IntPtr pUnkOuter, uint dwClsContext, ref Guid riid, out IntPtr ppv);

    public delegate int DllGetClassObjectDelegate(ref Guid rclsid, ref Guid riid, out IntPtr ppv);
    // Keep delegate references alive so the GC does not collect them mid-call.
    public static DllGetClassObjectDelegate _dgco;

    public static string Load(string dll) {
        IntPtr h = LoadLibraryW(dll);
        if (h == IntPtr.Zero) return "LoadLibrary FAILED, GetLastError=" + Marshal.GetLastWin32Error();
        return "OK handle=0x" + h.ToString("X");
    }
    public static string Instantiate(string dll, string clsid, string iidProvider) {
        // Ensure the DLL is loaded into this process first.
        IntPtr h = LoadLibraryW(dll);
        if (h == IntPtr.Zero) return "LoadLibrary FAILED, GetLastError=" + Marshal.GetLastWin32Error();
        // Verify DllGetClassObject export resolves (sanity check).
        IntPtr p = GetProcAddress(h, "DllGetClassObject");
        if (p == IntPtr.Zero) return "GetProcAddress(DllGetClassObject) FAILED, err=" + Marshal.GetLastWin32Error();
        // Activate via CoCreateInstance (CLSCTX_INPROC_SERVER = 1).
        Guid c = new Guid(clsid);
        Guid iP = new Guid(iidProvider);
        IntPtr prov;
        int hr = CoCreateInstance(ref c, IntPtr.Zero, 1, ref iP, out prov);
        if (hr != 0 || prov == IntPtr.Zero) return "CoCreateInstance hr=0x" + hr.ToString("X8");
        return "SUCCESS: provider=0x" + prov.ToString("X");
    }
}
'@
try {
    Add-Type -TypeDefinition $source -Language CSharp -ErrorAction Stop
    $loadRes = [CPDiag]::Load($dllPath)
    Log "  LoadLibrary: $loadRes"
    $instRes = [CPDiag]::Instantiate($dllPath, $clsid, "d27c3481-5a1c-45b2-8aaa-c20ebbe8229e")
    Log "  COM Instantiate: $instRes"
    if ($instRes -like "SUCCESS*") {
        Log "  => DLL/COM layer is HEALTHY. If tile still missing, problem is in LogonUI policy/enumeration/trigger."
    } else {
        Log "  => DLL/COM layer FAILED. Fix the DLL/registration per the HRESULT above."
    }
} catch {
    Log "  Diagnostic harness error: $_"
}

# --- 4. Policy / exclusion checks --------------------------------
Log ""
Log "[4] Policy / exclusion checks"
$polPath = "HKLM:\SOFTWARE\Policies\Microsoft\Windows\System"
if (Test-Path $polPath) {
    $pol = Get-ItemProperty $polPath
    Log "  Policies\Windows\System EnumerateLocalUsers=$($pol.EnumerateLocalUsers) DontDisplayLastUserName=$($pol.DontDisplayLastUserName)"
} else {
    Log "  No Policies\Windows\System key (default behavior)"
}
$cpRoot = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers"
$ourKey = Get-ItemProperty "$cpRoot\$clsid" -ErrorAction SilentlyContinue
Log "  Our CP DisabledByDefault=$($ourKey.DisabledByDefault)  Policy=$($ourKey.Policy)"

Log ""
Log "  All registered Credential Providers:"
Get-ChildItem $cpRoot -ErrorAction SilentlyContinue | ForEach-Object {
    $p = Get-ItemProperty $_.PSPath -ErrorAction SilentlyContinue
    $name = $p.'(default)'
    $dis = $p.Disabled
    $disStr = if ($null -eq $dis) { "(no Disabled value)" } else { "$dis" }
    $short = $_.PSChildName
    Log ("    {0}  Disabled={1}  Name={2}" -f $short, $disStr, $name)
}

# --- 5. Trace log files ------------------------------------------
Log ""
Log "[5] Trace log files"
$traceCandidates = @(
    "C:\ProgramData\WinSLA\cp_trace.log",
    "$env:TEMP\WinSLA_cp_trace.log",
    "C:\Windows\Temp\WinSLA_cp_trace.log"
)
$found = $false
foreach ($t in $traceCandidates) {
    if (Test-Path $t) {
        $found = $true
        Log "  FOUND: $t"
        Log "  ---- content ----"
        Get-Content $t | ForEach-Object { Log "  $_" }
        Log "  -----------------"
    }
}
if (-not $found) {
    Log "  No trace file found in any candidate location."
    Log "  (If step [3] COM Instantiate = SUCCESS but no trace here, check write permissions.)"
}

Log ""
Log "=================================================="
Log "Diagnostic complete. Report saved to $report"
Log "=================================================="
