# 开发指南

## telegram bot

为了最高的性能和安全性，后端完全采用 Rust 开发。

1. Install [Rust].
2. Setup your bot with [@botfather](https://t.me/botfather).
3. Clone this repository.
4. Set the environment variables:
   ```sh
   export TELOXIDE_TOKEN=<BOT TOKEN e.g. 123456789:ABCDEFGHIJKLMNOPQRSTUVWXYZ>
   export TELOXIDE_PROXY=<PROXY e.g. http://127.0.0.1:7890>
   ```
5. Run `cargo run` from the repository directory.
6. Send a message to your bot with `/start` command.
7. Enjoy!

## web

目前：完全前后端分离，前端使用完全静态的界面，后端只提供 API

如果继续开发 web 端，则需要重构目前提供的 demo

## 核心库 `lib`

### 数据库 `db`

### 加密与安全 `sec`

## 弱中心

目前还仍然只是构想阶段，目前可以用以下*极其临时*的方法创建新中心：

1. 获取可执行文件与数据库文件
2. 向 [@botfather](https://t.me/botfather) 申请并运行 bot
3. 在 [@SAFC_group](https://t.me/SAFC_group) 声名您的 bot
4. 定期同步数据库

## TODOs

tg bot 的功能比较受限，是否应该大力开发 web 端？

- VIS
  - [ ] logo
- db
  - [ ] 使用更现代化的 sql 范式
  - [ ] 分离与定时备份、发布
- tg bot 功能
  - [x] `/start` 重构 —— 作为功能指引
  - [x] 嵌套评价
    - [x] 更方便优雅地评价（翻页、回调）
    - [ ] 输出可能长于 4096，超出单条消息上线
  - [x] 模糊/快速 搜索 - 转为内联按钮的形式
  - [ ] 词云？关键字提取？
  - [x] 数据汇报
  - [ ] 抗攻击 - 按 uid 限制次数
  - [ ] 数据定时上传备份到 @SAFC_group
  - [ ] 向管理员发送日志
- web
  - [x] 静态网页 demo
  - [ ] 完整功能
- 更多平台
  - [ ] 浏览器插件 方便地加入导师相关评价
  - [ ] Discord、matrix 等更多社群平台
- 部署
  - [ ] env 转而使用配置文件的形式 & docker
- 数据
  - [ ] wiki 形式的客体基本信息
- 文档
  - [ ] 开发文档
  - [ ] 使用文档，包括导师评价规范、隐私目的的文字指导、社区公约等
- **带计划！**
  - [ ] 基于 Telegram 通讯的分布式数据库与分布式 bot
