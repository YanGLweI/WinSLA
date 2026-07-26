# WinSLA Credential Provider 注册脚本
# 以管理员身份运行此 PowerShell 脚本

$clsid = "{E4D9F6E7-8A2B-4C3D-9E5F-1A2B3C4D5E6F}"
$instDir = "C:\Program Files (x86)\WinSLA"
$dllPath = "$instDir\DualAuthCP.dll"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "WinSLA Credential Provider 注册脚本" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# 验证 DLL 存在
Write-Host "[1/5] 验证 DLL 文件..." -ForegroundColor Yellow
if (-not (Test-Path $dllPath)) {
    Write-Host "ERROR: DLL 文件不存在于 $dllPath" -ForegroundColor Red
    Write-Host "请检查 DLL 是否已正确复制到安装目录" -ForegroundColor Red
    exit 1
}
Write-Host "✓ DLL 文件已找到" -ForegroundColor Green

# 获取当前日期时间作为日志标记
$dateStamp = Get-Date -Format "yyyy-MM-dd HH:mm:ss"
Write-Host ""
Write-Host "[2/5] 正在创建注册表项 ($dateStamp)..." -ForegroundColor Yellow

try {
    # 1. 创建 CLSID 主键
    Write-Host "  [1/3] 创建 CLSID 基础键..." -ForegroundColor Gray
    New-Item -ItemType RegistryKey -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid" -Force | Out-Null
    
    # 设置描述
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid" -Name "(default)" -Value "WinSLA Dual-Auth Credential Provider" -Force
    Write-Host "      ✓ 已创建 CLSID 主键" -ForegroundColor Green
    
    # 2. 创建 InprocServer32 子键
    Write-Host "  [2/3] 配置 InprocServer32..." -ForegroundColor Gray
    New-Item -ItemType RegistryKey -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Force | Out-Null
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Name "(default)" -Value $dllPath -Force
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$clsid\InprocServer32" -Name "ThreadingModel" -Value "Apartment" -Force
    Write-Host "      ✓ 已配置 InprocServer32" -ForegroundColor Green
    
    # 3. 注册到 Credential Providers
    Write-Host "  [3/3] 注册到 Credential Providers..." -ForegroundColor Gray
    New-Item -ItemType RegistryKey -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Force | Out-Null
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Name "(default)" -Value "WinSLA Dual-Auth" -Force
    Set-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$clsid" -Name "Disabled" -Value 0 -Type DWord -Force
    Write-Host "      ✓ 已注册到 Credential Providers" -ForegroundColor Green
    
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "注册成功完成！" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    
    # 显示最终配置
    Write-Host "最终配置:" -ForegroundColor Cyan
    Write-Host "  CLSID:       $clsid"
    Write-Host "  DLL 路径：    $dllPath"
    Write-Host "  ThreadingModel: Apartment"
    Write-Host "  Disabled:     0 (启用)"
    Write-Host ""
    
    Write-Host "下一步操作：" -ForegroundColor Yellow
    Write-Host "1. 重启计算机或注销后重新登录" -ForegroundColor White
    Write-Host "2. 观察登录界面是否出现双账号认证 tile" -ForegroundColor White
    Write-Host ""
    
    # 提供快速重启选项
    $response = Read-Host "`n是否立即重启计算机？(Y/N)"
    if ($response -eq "Y" -or $response -eq "y") {
        Write-Host "正在关闭系统..." -ForegroundColor Cyan
        Start-Sleep -Seconds 2
        shutdown /r /t 0
        exit 0
    } else {
        Write-Host "已取消重启，请手动重启计算机以应用更改。" -ForegroundColor Cyan
    }
    
} catch {
    Write-Host ""
    Write-Host "ERROR: 注册表操作失败！" -ForegroundColor Red
    Write-Host "错误信息：$($_.Exception.Message)" -ForegroundColor Red
    Write-Host ""
    Write-Host "请检查：" -ForegroundColor Yellow
    Write-Host "- 是否以管理员身份运行 PowerShell" -ForegroundColor White
    Write-Host "- 注册表权限是否正确" -ForegroundColor White
    exit 1
}
