# frostflake-rs

[莫娜占卜铺](https://www.mona-uranai.com/) 使用 [YAS](https://github.com/wormtql/yas) 扫描圣遗物。[霜华(frostflake)](https://github.com/YuehaiTeam/frostflake) 提供了从网页拉起 YAS 的能力，免去了命令行的麻烦。

可惜霜华已经许久未更新，因此我尝试写一个简易的 Rust 替代实现。

## 可能导致的问题

- 霜华 [使用注册表提供 URI 启动的功能](https://learn.microsoft.com/en-us/previous-versions/windows/internet-explorer/ie-developer/platform-apis/aa767914(v=vs.85))，本程序为了完全替代，会覆盖该注册表项。 
