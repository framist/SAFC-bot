// use teloxide::prelude::*;
// use teloxide::utils::markdown::escape;
use teloxide::types::InlineKeyboardButton;
use teloxide::types::InlineKeyboardMarkup;

const BOT_INFO: &str = r#"# 大学生反诈中心

*社群，保护，开放*

## 背景

自从最初的导师评价网（urfire）关闭，时至今日，一批一批的新导师评价数据分享平台的迭起兴衰，最终都落于 404 或收费闭塞。
不知是何等阻力，让受过欺骗的学生和亟需信息的学生散若渺茫星火。
故建此平台与机器人，革新方式，坚持“社群，保护，开放”的理念，信奉密码朋克、开源精神，愿此和谐共赢地持久性发展传承下去。

## 目的

除了警惕那些专业的反诈人员，那些大学生最容易信任的客体才是最危险的

为了最大保护信息安全与隐私，大学生反诈中心（SAFC）基于 telegram 平台，包含以下功能

* telegram 机器人 @SAFC_bot —— 学校、专业、学院、课程、导师的交叉评价与查询
* telegram 群组社区 @SAFC_group —— 公告与交流平台

本平台遵守几点为主旨：

* 出发：共享，开放，自由的精神；我为人人，人人为我的理念
* 技术：密码朋克，尽可能地做好隐私保护、数据与人身安全；数据共享代码开源，相互监督共进。
* 定位：综合大学生所需要的功能，不光包括最基本的导师评价和查询功能，还能对学校、专业、学院、课程、学生、已有的评价进行评价；另外提供一个交流平台。

## 隐私

- 为防止滥用，你的 uid 可能会被临时储存在内存中，最多 1 日，除此之外不会记录任何个人信息。
- 「发布人 OTP」是可以让您日后证明本评价由您发布，由此您可以修改/销毁此评论。其非必选项，且仅会储存其加盐哈希。
- 我们默认 Telegram 是可信及安全的
- 早期开发结束后，代码与数据将完全开源

## 发展

目前敏捷性开发，以功能上线时间为先，后续需要大量的开发重构。

## 参考

初始数据来源：

https://github.com/pengp25/RateMySupervisor

https://gitee.com/wdwdwd123/RateMySupervisor.git

"#;

pub enum TgResponse {
    Hello,
    Info,
    RetryErrNone,
    NotImplemented,
}

impl ToString for TgResponse {
    fn to_string(&self) -> String {
        // escape(&self.to_unescaped_string())
        match self {
            Self::Hello => concat!(
                "嗨！我是大学生反诈中心的客服机器人 👋\n",
                "_目前仍为早期开发版本_ 问题敬请反馈；*越墙不易，延迟丢包敬请见谅*\n",
                "您可以发送 /cancel 来停止此次对话\n\n",
                "您可以先查询客体，然后查看或发起对客体的评价。\n\n",
                "您想查询或评价的「学校类别」是？您可以直接输入或者在下面的键盘选择框中选择\n\n",
                "_键盘选择框中没有的也可以直接输入来新建；如果是上个类别本身请选择或输入 `self`。下同_\n",
                "（如果是在 PC 端群聊中使用，键盘选择框弹出可能有 bug）",        
            )
            .to_owned(),
            Self::Info => BOT_INFO.to_owned(),
            Self::RetryErrNone => "空消息错误。对不起，请重试".to_owned(),
            Self::NotImplemented => "😢 功能尚未实现，敬请期待".to_owned(),
        }
    }
}

// TODO 使用序列化与反序列化实现 代替 EnumString
use strum_macros::{Display, EnumString};
#[derive(Debug, EnumString, Display)] // ?
pub enum ObjectOp {
    Read,
    Commet,
    Info,
    End,
    Add,
    Return(i32),
}

impl Into<String> for ObjectOp {
    fn into(self) -> String {
        format!("{:?}", self)
    }
}

pub fn build_op_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        [
            InlineKeyboardButton::callback("👀 查看评价", ObjectOp::Read),
            InlineKeyboardButton::callback("💬 增加评价", ObjectOp::Commet),
        ],
        [
            InlineKeyboardButton::callback("🤗 详细信息", ObjectOp::Info),
            InlineKeyboardButton::callback("🏁 结束", ObjectOp::End),
        ],
    ])
}

#[test]
fn my_test() {
    use std::str::FromStr;
    println!("{:?}", ObjectOp::Return(2));
    match ObjectOp::from_str("Return").unwrap() {
        ObjectOp::Read => {}
        ObjectOp::Commet => {}
        ObjectOp::Info => todo!(),
        ObjectOp::End => todo!(),
        ObjectOp::Add => todo!(),
        ObjectOp::Return(i) => println!("Return{i}"),
    }
}
