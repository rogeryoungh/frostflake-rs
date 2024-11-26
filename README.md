# frostflake-rs

[frostflake-rs](https://github.com/rogeryoungh/frostflake-rs) 是一个用于替代 [霜华 (frostflake)](https://github.com/YuehaiTeam/frostflake) 的轻量级 Rust 实现，它与 [YAS](https://github.com/wormtql/yas) 配合使用，简化了 [莫娜占卜铺](https://www.mona-uranai.com/) 的圣遗物扫描操作，省去了命令行的繁琐步骤。

由于原始的霜华项目已长期未更新，此项目旨在提供一个简易的替代方案，确保功能的可用性和便利性。

## 下载

> [!WARNING] 
> 尚在测试中。

- Git 最新构建， [Development Build](https://github.com/rogeryoungh/frostflake-rs/releases/tag/latest)
- 稳定构建，[GitHub Release](https://github.com/rogeryoungh/frostflake-rs/releases)。

## 注意事项

- **注册表覆盖**：霜华通过 [注册表 URI 协议](https://learn.microsoft.com/en-us/previous-versions/windows/internet-explorer/ie-developer/platform-apis/aa767914(v=vs.85)) 启动 YAS，本程序也采用相同的方式，安装时会覆盖相关注册表项。
- **YAS 下载缓慢**：本程序会自动从 YAS 的 GitHub Release 页面下载最新版本，由于众所周知的原因，下载过程可能较慢。
- **安全性限制**：由于 GitHub Release 不提供校验码，程序无法验证下载的 YAS 是否被篡改。为提升安全性，建议将本程序安装到 `C:\Program Files` 等受保护目录，因为该目录的文件编辑需要管理员权限。

## 待办

- [x] 下载进度条。
- [ ] 优化可执行文件大小。
- [ ] Toast 授权 token 按钮。
