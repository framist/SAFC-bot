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
   export SAFC_DB_PATH=<DATABASE PATH e.g. /path/to/safc.db>
   ```
5. Run `cargo run` from the repository directory.
6. Send a message to your bot with `/start` command.
7. Enjoy!

作为系统服务运行参考：

```toml
[Unit]
Description=SAFC Bot Service
After=network.target

[Service]
Environment="https_proxy=http://127.0.0.1:7890"
Environment="http_proxy=http://127.0.0.1:7890"
Environment="TELOXIDE_TOKEN=123456789:ABCDEFGHIJKLMNOPQRSTUVWXYZ"
Environment="TELOXIDE_PROXY=http://127.0.0.1:7890"
Environment="SAFC_DB_PATH=/path/to/db.sqlite"
ExecStart=/path/to/safc_bot
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## web

目前：完全前后端分离，前端使用完全静态的界面，后端只提供 API

如果继续开发 web 前端，则需要重构目前提供的 demo，在一个新的仓库分离开发

## 核心库 `lib`

### 数据库 `db`

数据库需要完全彻底的重构，但具体的实现方案仍未妥善设计。重新设计的数据库需满足去中心化的特征。

### 加密与安全 `sec`

## 弱中心

目前还仍然只是构想阶段，目前可以用以下*极其临时*的方法创建新中心：

1. 获取可执行文件与数据库文件
2. 向 [@botfather](https://t.me/botfather) 申请并运行 bot
3. 在 [@SAFC_group](https://t.me/SAFC_group) 声名您的 bot
4. 定期同步数据库

## 元平台

目前仍是通过手动运行脚本来完成这个特征。仍有很多代码内外的事情需要考虑。

## TODOs

目前的重点工作：数据库需要完全彻底的重构

- VIS
  - [ ] logo
- db
  - [ ] **使用更现代化的 sql 范式**
  - [ ] **定时备份、发布数据库**
- tg bot 功能
  - [x] `/start` 重构 —— 作为功能指引
  - [x] 嵌套评价
    - [x] 更方便优雅地评价（翻页、回调）
    - [ ] 输出可能长于 4096，超出单条消息上线
  - [x] 模糊/快速 搜索 - 转为内联按钮的形式
  - [ ] 评价的编辑与删除
  - [x] 数据汇报
  - [ ] 抗攻击 - 按 uid 限制次数
  - [ ] 数据定时上传备份到 @SAFC_group *目前高优先级*
  - [ ] 向管理员发送日志
- web
  - [x] 静态网页 demo
    - [ ] GitHub Pages CD
  - [ ] 完整功能
- 更多平台
  - [ ] 浏览器插件 方便地加入导师相关评价
  - [ ] **Discord、matrix 等更多社群平台**
- 部署
  - [ ] env 转而使用配置文件的形式 & docker
  - [ ] CI CD 自动部署
- 数据
  - [ ] wiki 形式的客体基本信息
- 文档
  - [ ] 开发文档
  - [ ] 使用文档，包括导师评价规范、隐私目的的文字指导、社区公约等
- **带计划！**
  - [ ] **基于 Telegram 通讯的分布式数据库与分布式 bot**
    - [ ] 目前的想法是使用区块链类似的结构
