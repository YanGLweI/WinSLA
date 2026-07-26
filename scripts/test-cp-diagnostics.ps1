# Test Credential Provider Registration and Functionality
# Run as Administrator to verify CLSID and DLL exports

$clsid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"
$dllPath = "C:\Program Files (x86)\WinSLA\DualAuthCP.dll"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "Credential Provider Diagnostics" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# 1. Verify DLL exists
Write-Host "[Step 1] Checking DLL file..." -ForegroundColor Yellow
if (-not (Test-Path $dllPath)) {
    Write-Host "ERROR: DLL not found at $dllPath" -ForegroundColor Red
    exit 1
}
Write-Host "✓ DLL exists: $($dllPath)" -ForegroundColor Green

# 2. Check registry entries
Write-Host "`n[Step 2] Verifying registry keys..." -ForegroundColor Yellow

$regPaths = @(
    "HKLM:\SOFTWARE\Classes\CLSID\$clsid",
    "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32",
    "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"
)

foreach ($path in $regPaths) {
    if (Test-Path $path) {
        Write-Host "✓ Registry key exists: $path" -ForegroundColor Green
    } else {
        Write-Host "✗ Missing registry key: $path" -ForegroundColor Red
    }
}

# 3. Get dumpbin.exe path
Write-Host "`n[Step 3] Checking DLL exports with dumpbin..." -ForegroundColor Yellow
$dumpbinPaths = @(
    "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe",
    "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe",
    "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe",
    "C:\Program Files (x86)\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\*\bin\Hostx64\x64\dumpbin.exe"
)

$dumpbin = $null
foreach ($path in $dumpbinPaths) {
    $found = Get-ChildItem -Path $path -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) {
        $dumpbin = $found.FullName
        break
    }
}

if (-not $dumpbin) {
    Write-Host "ERROR: Could not find dumpbin.exe" -ForegroundColor Red
    Write-Host "Please install Visual Studio Build Tools" -ForegroundColor Yellow
} else {
    Write-Host "Using: $dumpbin" -ForegroundColor Gray
    try {
        $exports = & $dumpbin /exports $dllPath 2>&1 | Out-String
        Write-Host "`n--- Export Table ---" -ForegroundColor Cyan
        Write-Host $exports
        
        # Check for required exports
        if ($exports -match "DllGetClassObject") {
            Write-Host "`n✓ DllGetClassObject found" -ForegroundColor Green
        } else {
            Write-Host "`n✗ ERROR: DllGetClassObject NOT FOUND!" -ForegroundColor Red
            Write-Host "Credential Provider will NOT load without this export." -ForegroundColor Yellow
        }
        
        if ($exports -match "DllCanUnloadNow") {
            Write-Host "✓ DllCanUnloadNow found" -ForegroundColor Green
        } else {
            Write-Host "⚠ DllCanUnloadNow NOT FOUND (warning but may work)" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "Failed to run dumpbin: $_" -ForegroundColor Red
    }
}

# 4. List registered Credential Providers
Write-Host "`n[Step 4] Listing all registered Credential Providers..." -ForegroundColor Yellow
$cpDir = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers"
if (Test-Path $cpDir) {
    Get-ChildItem -Path $cpDir -ErrorAction SilentlyContinue | ForEach-Object {
        $name = $_.GetValue("", "")
        Write-Host "  CLSID: $($_.Name.Split('\')[-1])`t Name: $name" -ForegroundColor Gray
    }
} else {
    Write-Host "No Credential Providers registered" -ForegroundColor Yellow
}

Write-Host "`n=========================================" -ForegroundColor Cyan
Write-Host "Diagnostic Complete" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
