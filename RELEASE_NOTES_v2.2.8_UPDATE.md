# WinSLA v2.2.8 - Rust Edition 配置修复

**发布日期**: 2026-09-10  
**重要提示**: 版本号的二进制文件嵌入错误已修复

---

## 🔧 Bug 修复清单

### ⚠️ 核心问题修复：版本号显示不正确 ❌→✅

#### 问题描述

仪表盘界面始终显示 **v2.2.6**,尽管 Cargo.toml 已经更新为 **v2.2.8**。

**影响范围**: 
- 所有用户安装的 WinSLA 管理端
- 仪表盘中服务状态显示错误版本号
- 安装包元数据不一致

#### 根本原因分析

**发现两个相互关联的问题:**

1. **Rust edition 配置错误**
   ```toml
   # 错误配置 (cp_provider/Cargo.toml)
   edition = "2026"  # ← 不存在的 edition
   ```
   
   Windows 上的 Rust Cargo 工具链版本只支持以下 editions:
   - ✅ 2015
   - ✅ 2018  
   - ✅ 2021
   - ✅ 2024
   
   `edition = "2026"` 在当时的 Rust 版本中不存在！

2. **增量编译缓存未清理**
   - target 目录中包含旧版本的编译产物
   - Cargo 未检测到版本变更（因为 edition 错误导致无法重新编译）

#### 技术细节

**Rust env! 宏的工作机制**:
```rust
// 在 server.rs 中读取版本号
version: env!("CARGO_PKG_VERSION").to_string()
```

`env!("CARGO_PKG_VERSION")` 是在**编译时**从 Cargo.toml 读取的值，并**硬编码到二进制文件中**。

一旦编译完成，二进制文件中的版本号就无法改变，除非重新编译。

#### 解决方案

**Step 1: 修复 edition 配置**

```diff
# cp_provider/Cargo.toml
-edition = "2026"
+edition = "2021"

# win_service/Cargo.toml  
-edition = "2026"
+edition = "2021"
```

**Step 2: 清理目标目录强制重新编译**

```powershell
cd C:\Users\YLW\Documents\PJ\WinSLA
Remove-Item -Path ".\management_app\target" -Recurse -Force
Remove-Item -Path ".\target" -Recurse -Force
cargo build --release
```

**Step 3: 验证编译成功**

```
Finished release profile [optimized] in 1m 17s
winsla-management.exe: 8,299,520 bytes (8.3 MB)
```

**Step 4: 重建 NSIS 安装包**

```powershell
.\scripts\build_installer.ps1
# Output: WinSLA-v2.2.8-Setup.exe (5.26 MB)
```

---

## 📊 对比表格

| 项目 | 修复前 | 修复后 |
|-----|-------|--------|
| Cargo.toml edition | "2026" (错误) | "2021" (正确) |
| winsla-management.exe 版本 | v2.2.6 (缓存) | v2.2.8 (新编译) |
| 安装包版本号 | v2.2.7 (NSIS) | v2.2.8 (NSIS) |
| 仪表盘显示 | "版本 2.2.6" ❌ | "版本 v2.2.8" ✅ |
| 编译状态 | 失败 (edition 错误) | 成功 ✅ |

---

## 💡 经验总结

### 教训与最佳实践

1. **Cargo.toml 版本管理的注意事项**
   - version 字段更新后必须重新编译才能生效
   - 检查 edition 是否与支持的工具链兼容
   - 使用 `cargo --version` 确认当前 Rust 版本支持的 editions

2. **增量编译陷阱**
   - 仅修改 Cargo.toml 可能不会触发生效
   - 需要确保 target 目录已清理或使用 `--force-rebuild` (如果支持)
   - 建议在 CI/CD 流程中每次都清理目标目录后再构建

3. **版本号验证方法**
   ```bash
   # 方式 1: 检查二进制文件属性
   Get-ItemProperty .\target\release\winsla-management.exe | Select-Object VersionInfo
   
   # 方式 2: 运行程序查看版本
   .\target\release\winsla-management.exe --version
   
   # 方式 3: 在 GUI 界面查看仪表盘
   ```

---

## 🎯 修复后的完整性检查

### 版本号一致性验证

| 组件 | 版本号 | 验证方式 |
|-----|--------|----------|
| `cp_provider/Cargo.toml` | 2.2.8 | ✅ 查看文件内容 |
| `win_service/Cargo.toml` | 2.2.8 | ✅ 查看文件内容 |
| `management_app/Cargo.toml` | 2.2.8 | ✅ 查看文件内容 |
| `management_app/package.json` | 2.2.8 | ✅ 查看文件内容 |
| `installer/winsla-installer.nsi` | v2.2.8 | ✅ 查看脚本内容 |
| `winsla-management.exe` | v2.2.8 | ✅ 运行时版本显示 |
| `WinSLA-v2.2.8-Setup.exe` | v2.2.8 | ✅ 安装包文件名 |

---

## 🛡️ 兼容性说明

### 向后兼容性

✅ **完全向后兼容**
- 安装路径、注册表位置无变化
- 配置文件格式无变化  
- 现有功能不受影响
- 可平滑升级无需额外操作

### 环境要求

- ✅ Windows 10/11 64 位
- ✅ Rust toolchain v1.70+ (当前 v1.97.1)
- ✅ NSIS (makensis) v3.x

---

## 📋 升级指南

### 适用对象

所有正在使用 WinSLA v2.2.6 或 v2.2.7 的用户

### 建议操作

1. **备份当前配置**(可选):
   ```powershell
   Copy-Item "$env:LOCALAPPDATA\WinSLA\winsla.db" "backup\winsla-backup.db"
   ```

2. **卸载旧版本**:
   ```powershell
   # 通过控制面板或通过安装程序自带的卸载程序
   ```

3. **安装新版本**:
   - 下载: https://github.com/YanGLweI/WinSLA/releases/tag/v2.2.8
   - 运行 `WinSLA-v2.2.8-Setup.exe`
   - 授权管理员权限

4. **验证安装**:
   - 打开 WinSLA 管理端
   - 检查仪表盘 → 服务状态 → 应显示 **"版本 v2.2.8"**

---

## 🔗 相关链接

- **GitHub Releases**: https://github.com/YanGLweI/WinSLA/releases/tag/v2.2.8
- **Issue 报告**: https://github.com/YanGLweI/WinSLA/issues
- **完整文档**: https://github.com/YanGLweI/WinSLA/blob/master/README.md

---

## 🙏 致谢

感谢社区成员反馈的版本号不一致问题！

---

## 📜 许可证

MIT License - © 2026 ylw

---

*如有任何问题，请提交 Issue 或在讨论区留言。*
