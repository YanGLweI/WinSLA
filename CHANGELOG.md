# Changelog - WinSLA

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added
- Initial changelog structure

---

## [2.2.7] - 2026-09-09

### Fixed
- **Bug #1**: 应急账号提醒时机优化 - 从"创建主账号后立即弹出"改为"完成第一条完整配对关系配置后（添加至少一个审批人）再提示"
- **Bug #2**: JSON 反序列化错误 - 修复 Switch Toggle 状态切换时后端期望 `i64` 但接收到 `boolean` 导致的 422 错误
- **Bug #3**: 标签页切换自发消息 - 移除切换 Dashboard/Pairing Rules 时无缘无故弹出的成功/错误消息
- **Bug #4**: 删除最后一条配对未启用默认 Tile - 删除所有配对关系后自动调用 API 启用 Windows 默认登录 Tile，防止用户被锁死
- **Bug #5**: 新增主账号审批人对话框 422 错误 - 修复当已有配对关系时创建第二个主账号，自动弹出的审批人对话框因后端返回结构嵌套问题导致的提交失败

### Changed
- 管理端前端版本号从 `2.2.0` 升级到 `2.2.7`
- NSIS 安装包版本号更新为 `v2.2.7`
- 优化 `handleAddAccount` 函数，正确处理第一条配对的嵌套响应和后续配对的直接响应

---

## [2.2.6] - 2026-08-XX

### Fixed
- HTTP 422 错误：switch toggle 按钮点击后报错 `invalid type: boolean 'false', expected i64`
- 弹窗误触发问题

---

## [2.2.5] - 2026-XX-XX

### Added
- 密码过期处理功能
