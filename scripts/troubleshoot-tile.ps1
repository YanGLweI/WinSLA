# Complete Diagnostic Script for Dual-Account Authentication Tile Issue

$clsid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"
$dllPath = "C:\Program Files\WinSLA\DualAuthCP.dll"

Write-Host "=========================================" -ForegroundColor Cyan
Write-Host "WinSLA Dual-Account Tile Troubleshooting" -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""

# 1. Check DLL exports
Write-Host "[Step 1] Checking DLL exports..." -ForegroundColor Yellow
try {
    $dumpbin = Get-ChildItem "C:\Program Files (x86)\Microsoft Visual Studio" -Recurse -Filter "dumpbin.exe" | Select-Object -First 1 FullName
    if ($dumpbin) {
        $exports = & $dumpbin.FullName /exports $dllPath 2>&1 | Out-String
        
        if ($exports -match "DllGetClassObject") {
            Write-Host "✓ DllGetClassObject exists" -ForegroundColor Green
        } else {
            Write-Host "✗ ERROR: DllGetClassObject missing!" -ForegroundColor Red
            exit 1
        }
        
        if ($exports -match "DllCanUnloadNow") {
            Write-Host "✓ DllCanUnloadNow exists" -ForegroundColor Green
        } else {
            Write-Host "⚠ DllCanUnloadNow missing (may cause issues)" -ForegroundColor Yellow
        }
    }
} catch {
    Write-Host "Could not check exports: $_" -ForegroundColor Yellow
}

# 2. Check registry entries
Write-Host "`n[Step 2] Checking registry entries..." -ForegroundColor Yellow

$checks = @(
    @{ Path="HKLM:\SOFTWARE\Classes\CLSID\$clsid"; Name="(default)"; Expect="WinSLA Dual-Auth Credential Provider" },
    @{ Path="HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32"; Name="(default)"; Expect=$dllPath },
    @{ Path="HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32"; Name="ThreadingModel"; Expect="Apartment" },
    @{ Path="HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"; Name="(default)"; Expect="WinSLA Dual-Auth" },
    @{ Path="HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid"; Name="Disabled"; Type="DWord"; Expect="0" }
)

foreach ($check in $checks) {
    try {
        if (Test-Path $check.Path) {
            $value = Get-ItemProperty -Path $check.Path -Name $check.Name -ErrorAction SilentlyContinue
            if ($value) {
                if ($check.Type -eq "DWord") {
                    if ([int]$value.$($check.Name) -eq [int]$check.Expect) {
                        Write-Host "✓ $($check.Path.Split('\')[-1])`.$($check.Name) = $($value.$($check.Name))" -ForegroundColor Green
                    } else {
                        Write-Host "✗ $($check.Path.Split('\')[-1])`.$($check.Name) = $($value.$($check.Name)) (expected $($check.Expect))" -ForegroundColor Red
                    }
                } else {
                    if ($value.$($check.Name) -eq $check.Expect) {
                        Write-Host "✓ $($check.Path.Split('\')[-1])`.$($check.Name) matches" -ForegroundColor Green
                    } else {
                        Write-Host "✗ $($check.Path.Split('\')[-1])`.$($check.Name) = '$($value.$($check.Name))' (expected '$($check.Expect)')" -ForegroundColor Red
                    }
                }
            }
        } else {
            Write-Host "✗ Missing: $($check.Path)" -ForegroundColor Red
        }
    } catch {
        Write-Host "ERROR reading $($check.Path): $_" -ForegroundColor Red
    }
}

# 3. Check if other credential providers have priority
Write-Host "`n[Step 3] Checking CP ordering and visibility..." -ForegroundColor Yellow

$cpDir = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers"
if (Test-Path $cpDir) {
    Write-Host "Registered Credential Providers:" -ForegroundColor Gray
    Get-ChildItem -Path $cpDir -ErrorAction SilentlyContinue | ForEach-Object {
        $name = $_.GetValue("", "")
        $disabledProp = (Get-ItemProperty -Path $_.PSPath -Name "Disabled" -ErrorAction SilentlyContinue).Disabled
        if (-not $disabledProp) { $disabledProp = 1 }  # Default to Enabled if not set
        $status = if ($disabledProp -eq 0) { "Enabled" } else { "Disabled" }
        Write-Host "  CLSID: $($_.Name.Split('\')[-1]).Substring(0, 8)...`t Name: $name`t Status: $status" -ForegroundColor Gray
    }
}

# 4. Check if there's a DisableWithPasswordProvider setting
Write-Host "`n[Step 4] Checking CPDisableWithPasswordProvider..." -ForegroundColor Yellow
$globalRegKey = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\LogonUI"
if (Test-Path $globalRegKey) {
    try {
        $prop = Get-ItemProperty -Path $globalRegKey -Name "CredentialProviderExcludeByDefault" -ErrorAction SilentlyContinue
        Write-Host "⚠ Found 'CredentialProviderExcludeByDefault': $($prop.CredentialProviderExcludeByDefault)" -ForegroundColor Yellow
    } catch {
        Write-Host "No special exclusions found" -ForegroundColor Gray
    }
} else {
    Write-Host "Global LogonUI settings not found (normal on newer Windows versions)" -ForegroundColor Gray
}

# 5. Event log search
Write-Host "`n[Step 5] Checking event logs for errors..." -ForegroundColor Yellow
try {
    # Try to find recent LogonUI or Credential Provider related events
    $events = Get-WinEvent -LogName "System" -MaxEvents 50 -ErrorAction SilentlyContinue | 
        Where-Object { $_.Message -like "*Credential*" -or $_.Message -like "*LogonUI*" -or $_.Message -like "*CP*" -or $_.Source -like "*User32*" } |
        Select-Object -First 10 TimeCreated, Level, Message
    
    if ($events) {
        Write-Host "Recent events:" -ForegroundColor Yellow
        foreach ($event in $events) {
            Write-Host "[$($event.TimeCreated)] $($event.Level): $($event.Source)" -ForegroundColor Gray
            if ($event.Message.Length -gt 200) {
                Write-Host "  $($event.Message.Substring(0, 200))..." -ForegroundColor Gray
            } else {
                Write-Host "  $($event.Message)" -ForegroundColor Gray
            }
        }
    } else {
        Write-Host "No relevant events found" -ForegroundColor Gray
    }
} catch {
    Write-Host "Cannot read event logs: $_" -ForegroundColor Yellow
}

Write-Host "`n=========================================" -ForegroundColor Cyan
Write-Host "Troubleshooting complete." -ForegroundColor Cyan
Write-Host "=========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Next steps:" -ForegroundColor Yellow
Write-Host "1. Check event logs for errors during login attempt" -ForegroundColor White
Write-Host "2. Verify DLL can be loaded by checking with Dependency Walker" -ForegroundColor White
Write-Host "3. Try adding WinSLA CP explicitly in Group Policy (if available)" -ForegroundColor White
Write-Host "4. Consider implementing ICredentialProviderCredential::GetSerialization properly" -ForegroundColor White
Write-Host ""
