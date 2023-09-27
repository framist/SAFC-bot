<div align="center">
  <h1>🏛️</h1>
  <!-- <img width="150" heigth="150" src="./doc/asserts/icon.png"> -->
  <h1>SAFC</h1>
  <b>🧪 in developing but already in servering!</b><br/>
  <i>社群，安全，开源——不只是评价导师</i><br/>
  <!-- <a href="https://t.me/SAFC_bot"><del>Telegram 机器人</del></a> | --> 
  <a href="https://framist.github.io/safc">Web</a> |
  <a href="https://t.me/SAFC_bak_bot">Telegram 机器人</a> |
  <a href="https://t.me/SAFC_group">群组社区</a><br/> 
</div>

# 大学生反诈中心 SAFC


<details>
<summary>Demo</summary>
<img src="./assets/bot_demo.webp">
</details>


## 背景

自从最初的导师评价网（urfire）关闭，时至今日，一批一批的新导师评价数据分享平台的迭起兴衰，最终都落于 404 或收费闭塞。
不知是何等阻力，让受过欺骗的学生和亟需信息的学生散若渺茫星火。
故建此平台与机器人，革新方式，坚持“社群，保护，开放”的理念，信奉密码朋克、开源精神，愿此和谐共赢地持久性发展传承下去。

## 目标

除了警惕那些专业的反诈人员，那些大学生最容易信任的客体才是最危险的。

为了最大保护信息安全与隐私，大学生反诈中心（SAFC）基于 Telegram 平台，包含以下功能

* [Web 端](https://framist.github.io/safc)，只包括基本和静态的功能
* Telegram 群组社区 [@SAFC_group](https://t.me/SAFC_group) —— 公告与交流平台 *本仓库未来可能会因为各种原因失踪，请加入此群组以防迷路*
* Telegram 机器人 ~~[@SAFC_bot](https://t.me/SAFC_bot)~~ [@SAFC_bak_bot](https://t.me/SAFC_bak_bot) —— 学校、专业、学院、课程、导师的交叉评价与查询

本平台的长期目标：元平台、分布式

* _出发_：共享，开放，自由的精神；我为人人，人人为我的理念。
* _开源_：SAFC 的功能与基础设施将保证**永远免费，数据、代码开源**。源代码也必须是可读的，这样用户就可以理解系统真正在做什么，并更容易地扩展它。
* _技术_：密码朋克，尽可能地做好隐私保护、数据与人身安全；数据共享代码开源，相互监督共进；**未来预计实现完全的去中心化，每个人都能建立起新的节点**。
* _定位_：综合大学生所需要的功能，不光包括最基本的导师评价和查询功能，还能对学校、专业、学院、课程、学生、已有的评价进行评价；另外提供一个交流平台。
* _元平台_：**引用综合其他相似平台**。[详细](./script/crawlers/README.md)
* _价值观_：不审核把控评价，因为每个评价必有片面主观的地方。只有评价，没有评分，每个客体都不能由单独的分数来决定。

## 隐私

- 为防止滥用，您的 telegram uid 可能会被临时储存在内存中，最多 1 日，除此之外不会记录任何个人信息。
- 「发布人 OTP」是可以让您日后证明本评价由您发布，由此您可以修改/销毁此评论。其非必选项，且仅会储存其加盐哈希。
- 我们默认 Telegram 是可信及安全的
- 代码与数据将完全开源

## 发展

目前仍处开发初期阶段，以功能上线时间为先。*永远的 beta 版*

除了用户主动提交信息，我们也会尽可能地搜集、爬取可靠信息，让数据库更加全面。

*带计划：去中心化 —— 星星之火 可以燎原*

## 参考

<details>
<summary>数据来源：</summary>
见 `safc::db::SourceCate`
</details>

<details>
<summary>参考项目：</summary>
开发参考： 
前端页面魔改一下 RateMySupervisor 的，主要做后端和 API
</details>

<details>
<summary>目前还活着的相似站点：</summary>
- [PI Review - 研究生导师点评网站](https://pi-review.com/)
  - 功能齐全，也支持嵌套评价，使用体验好，UI 优良
  - 国际化，非营利；完全是自己的数据
  - 采用校园邮箱或邀请认证学生
  - 主要是 utsc 的用户比较多，但是评价、用户少
- [导师推荐人](https://mysupervisor.org/) 、https://ratemysupervisor.org/
  - 功能残缺
  - 付费站点
- [GRADPI](https://www.gradpi.com/)
  - 英文站点
</details>

# 开发

参见 [develop.md](./doc/develop.md)

---

[框架科工](https://craft.framist.top/) | 致力为虚无的世间献上一点花火🔥
