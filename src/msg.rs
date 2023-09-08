// use teloxide::prelude::*;
// use teloxide::utils::markdown::escape;
use teloxide::types::InlineKeyboardButton;
use teloxide::types::InlineKeyboardMarkup;

use serde::{Deserialize, Serialize};

const BOT_INFO: &str = r#"*大学生反诈中心*

_社群，保护，开放_

自从最初的导师评价网（urfire）关闭，时至今日，一批一批的新导师评价数据分享平台的迭起兴衰，最终都落于 404 或收费闭塞。
不知是何等阻力，让受过欺骗的学生和亟需信息的学生散若渺茫星火。故建此平台与机器人，革新方式，坚持“社群，保护，开放”的理念，信奉||密码朋克||、开源精神，愿此和谐共赢地持久性发展传承下去。

*telegram 机器人* @SAFC\_bot —— 学校、专业、学院、课程、导师的交叉评价与查询
*telegram 群组社区* @SAFC\_group —— 公告与交流平台

[GitHub 项目主页](https://github.com/framist/SAFC-bot)

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

/// 流程
/// 这个数据结构写得太烂了，有待优化
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub enum State {
    #[default]
    Start,
    SchoolCate,
    University {
        school_cate: String,
    },
    Department {
        school_cate: String,
        university: String,
    },
    Supervisor {
        school_cate: String,
        university: String,
        department: String,
    },
    Read {
        school_cate: String,
        university: String,
        department: String,
        supervisor: String,
        object_id: String,
    },
    Comment {
        school_cate: String,
        university: String,
        department: String,
        supervisor: String,
        object_id: String,
    },
    Publish {
        object_id: String,
        comment: String,
        comment_id: String,
        date: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ObjectOp {
    Read,
    Commet,
    Info,
    End,
    Add,
    // 最长只能 64 字符，所以选择这种 hack 的方法，有待改进
    ReturnU,
    ReturnD,
    ReturnS,
}

impl From<ObjectOp> for String {
    fn from(val: ObjectOp) -> Self {
        serde_json::to_string(&val).unwrap()
    }
}

// impl TryFrom<String> for ObjectOp {
//     type Error = serde_json::Error;
//     fn try_from(value: String) -> Result<Self, Self::Error> {
//         serde_json::from_str(&value)
//     }
// }

impl From<String> for ObjectOp {
    fn from(value: String) -> Self {
        serde_json::from_str(&value).unwrap()
    }
}

use serde_json;

pub fn build_op_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::callback(
                "👀 查看评价",
                serde_json::to_string(&ObjectOp::Read).unwrap(),
            ),
            InlineKeyboardButton::callback(
                "💬 增加评价",
                serde_json::to_string(&ObjectOp::Commet).unwrap(),
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                "🤗 详细信息",
                serde_json::to_string(&ObjectOp::Info).unwrap(),
            ),
            InlineKeyboardButton::callback(
                "🏁 结束",
                serde_json::to_string(&ObjectOp::End).unwrap(),
            ),
        ],
        vec![
            InlineKeyboardButton::callback(
                "↩️ 🏫",
                serde_json::to_string(&ObjectOp::ReturnU)
                .unwrap(),
            ),
            InlineKeyboardButton::callback(
                "↩️ 🏢",
                serde_json::to_string(&ObjectOp::ReturnD)
                .unwrap(),
            ),
            InlineKeyboardButton::callback(
                "↩️ 👔",
                serde_json::to_string(&ObjectOp::ReturnS)
                .unwrap(),
            ),
        ],
    ])
}

#[test]
fn my_test() {
    println!("{}", serde_json::to_string(&ObjectOp::Read).unwrap());
    // println!(
    //     "{}",
    //     serde_json::to_string(&ObjectOp::Return(State::Start)).unwrap()
    // );
    // println!(
    //     "{}",
    //     serde_json::to_string(&ObjectOp::Return(State::University {
    //         school_cate: "101".to_string()
    //     }))
    //     .unwrap()
    // );
    // println!(
    //     "{:#?}",
    //     InlineKeyboardButton::callback(
    //         "🏁 结束",
    //         ObjectOp::Return(State::University {
    //             school_cate: "101".to_string()
    //         })
    //     )
    // );
}
