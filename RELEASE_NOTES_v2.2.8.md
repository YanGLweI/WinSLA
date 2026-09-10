# WinSLA v2.2.8 版本发布说明

## 🎉 重要更新 (v2.2.8)

**发布日期**: 2026-09-10  
**版本号**: v2.2.8  
**主要焦点**: 配对规则禁用警告弹窗修复及安全机制完善

---

## 🐛 Bug 修复清单

### ⚠️ 严重问题修复 (Critical Fix)

#### 1. 禁用最后一条启用规则时无警告提示 ❌→✅

**问题描述**:  
在禁用最后一启用的配对规则时，系统缺少警告提示，可能导致用户误操作后无法使用双控登录功能，且不会自动启用 Windows 默认 Tile 作为安全兜底。

**根本原因**:  
Vue 的响应式系统 (`v-model`) 会在事件触发前立即同步更新 `row.enabled`的值，导致在事件处理器中检测不到禁用前的状态。

**技术方案**:  
采用**只读绑定模式**,通过`:model-value="row.enabled"`替代双向绑定的`v-model`,阻止 Vue 自动更新值，从而在事件处理器中正确获取旧状态。

```typescript
// Template 修改
<el-switch
  :model-value="row.enabled"  // 只读显示，不自动更新
  @change="(value) => handleToggleAccount(row, Boolean(value))"
/>

// Function 逻辑验证
if (!newEnabled && row.enabled) {  // 正确使用 row.enabled 进行判断
  // 检测到即将禁用最后一条启用规则 → 显示警告弹窗
  await ElMessageBox.confirm(...)
}
```

**影响范围**:  
所有用户在禁用配对规则时的用户体验及系统安全性

**修复验证**: ✅ 已测试通过

---

### 🔧 次要问题修复

#### 2. Vue 响应式系统状态同步错误

**问题描述**:  
在多次切换规则状态时，UI 与实际数据状态可能出现不同步的情况。

**修复内容**:  
优化了状态捕获机制，确保在事件处理函数的入口处就获取最新的实际状态值。

**改进点**:  
- 减少响应式依赖带来的副作用
- 提高状态读取的可靠性
- 增强代码的可维护性

---

## 💡 功能改进

### 1. 增强调试能力

新增了详细的调试日志输出，方便开发和问题排查：

```javascript
console.log('handleToggleAccount called:', { newEnabled, rowEnabled });

console.log('Debug toggle:', {
  otherEnabledRules,
  hasOtherCompletePair,
  shouldShowWarning: (otherEnabledRules === 0 || !hasOtherCompletePair),
  allAccounts: accounts.value.map(a => ({ 
    sid: a.account_sid.substring(0, 10), 
    enabled: a.enabled 
  }))
});
```

这些日志可以帮助：
- 快速定位状态同步问题
- 理解业务逻辑的执行流程
- 便于未来的问题排查

### 2. 前端资源缓存控制优化

#### index.html 改进
添加了标准的 HTTP 缓存控制 meta 标签，强制浏览器不使用缓存：

```html
<meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate" />
<meta http-equiv="Pragma" content="no-cache" />
<meta http-equiv="Expires" content="0" />
```

#### Vite 构建配置优化
为构建产物添加唯一的 hash 标识符，支持更好的缓存管理：

```javascript
// vite.config.ts
rollupOptions: {
  output: {
    entryFileNames: `assets/[name]-[hash].js`,
    chunkFileNames: `assets/[name]-[hash].js`,
    assetFileNames: `assets/[name]-[hash].[ext]`
  }
}
```

---

## 🔒 安全机制说明

### 配对规则禁用安全策略

本版本完善了配对规则禁用时的安全检查机制，具体包括：

#### 保护机制一：最后一条启用规则的警告
当用户尝试禁用最后一条启用的配对规则时，系统会：
1. ✅ 显示明确的警告提示
2. ✅ 告知用户后果（无法使用双控登录）
3. ✅ 说明系统将自动启用 Windows 默认 Tile 作为备用方案
4. ✅ 提供取消选项供用户重新考虑

#### 保护机制二：自动启用默认 Tile 兜底
如果系统中已没有任何启用的配对规则，系统会自动：
1. ✅ 启用 `default_tile_enabled = true`
2. ✅ 更新注册表确保立即生效
3. ✅ 通知用户备份登录方式的可用性

**示例场景**:
```
场景：禁用最后一对完整配对 (有审批人)
流程：
1. 用户点击禁用开关
2. 系统检查当前状态：only one complete pair is enabled
3. 弹出警告："这是最后一条启用的配对规则..."
4. 用户确认后继续禁用
5. 后端检测到无完整配对可用
6. 自动启用 default tile 并更新注册表
7. 用户仍可通过 Windows 默认方式登录
```

---

## 📋 升级指南

### 适用对象
所有正在使用 WinSLA 配对规则功能的用户

### 建议操作
- ✅ 建议立即升级到 v2.2.8 或更高版本
- ✅ 升级后请测试禁用配对规则的功能是否正常显示警告
- ✅ 确认 Windows 默认 Tile 的自动兜底机制是否正常工作

### 已知问题
暂无已知问题

### 向后兼容性
✅ 完全向后兼容，无需额外配置即可平滑升级

---

## 🙏 致谢

感谢以下贡献者和社区成员：
- 所有报告 bug 并提供反馈的用户
- 参与测试的社区成员
- 所有为项目做出贡献的开发者和使用者

---

## 📬 联系方式

- **GitHub Issues**: https://github.com/YanGLweI/WinSLA/issues
- **讨论区**: https://github.com/YanGLweI/WinSLA/discussions

---

## 📜 许可证

MIT License - © 2026 ylw

---

*感谢您选择 WinSLA!如有任何问题，请随时提交 Issue 或通过 Discord 与我们联系。*
