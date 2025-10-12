# 元平台

我们将**引用**其他相似平台的数据，吸纳归一。

我们会标注数据的原始来源与链接。
若违反规则，请联系我们。
也期待平台所有者能与我们合作，我们也会开放数据接口。

目前的数据来源请参见 `safc::db::SourceCate`，当前包含：

- `Admin`：由管理员手动整理录入。
- `Urfire`：来自 [Urfire](https://gitee.com/wdwdwd123/RateMySupervisor) 的数据（该仓库可能 fork 自 <https://github.com/pengp25/RateMySupervisor> 并略有更新）。
- `Telegram`：源自 SAFC 的 Telegram 平台互动数据。
- `Web`：计划来自 SAFC 的 Web 平台数据。
- `PiReview`：来自 [pi-review.com](https://pi-review.com/) 的数据，采集时间截止到 2023-09-26。

此文件夹下为一些爬取其他平台数据的脚本。

## TODO

- [ ] 一个浏览器插件，能方便地截取内容储存到 SAFC 数据库中。类如截取储存知乎「如何看到 xxx 老师」
