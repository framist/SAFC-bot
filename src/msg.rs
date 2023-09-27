// use teloxide::prelude::*;
// use teloxide::utils::markdown::escape;
use teloxide::types::InlineKeyboardButton;
use teloxide::types::InlineKeyboardMarkup;

use serde::{Deserialize, Serialize};

use safc::db::*;

// 有没有更优雅的方法？
use lazy_static::lazy_static;
lazy_static! {
    pub static ref SAFC_DB: SAFCdb = SAFCdb::new();
}

pub const GITHUB_URL: &str = "https://github.com/framist/SAFC-bot";

const BOT_INFO: &str = r"*大学生反诈中心*

_社群，保护，开放_

自从最初的导师评价网（urfire）关闭，时至今日，一批一批的新导师评价数据分享平台的迭起兴衰，最终都落于 404 或收费闭塞。
不知是何等阻力，让受过欺骗的学生和亟需信息的学生散若渺茫星火。故建此平台与机器人，革新方式，坚持“社群，保护，开放”的理念，信奉||密码朋克||、开源精神，愿此和谐共赢地持久性发展传承下去。

*telegram 机器人* ~@SAFC\_bot~ @SAFC\_bak\_bot —— 学校、专业、学院、课程、导师的交叉评价与查询
*telegram 群组社区* @SAFC\_group —— 公告与交流平台

[GitHub 项目主页](https://github.com/framist/SAFC-bot)
";

pub enum TgResponse {
    Hello,
    Info,
    RetryErrNone,
    #[allow(unused)]
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
                "请选择以下功能之一：",
            )
            .to_owned(),
            Self::Info => BOT_INFO.to_owned(),
            Self::RetryErrNone => "空消息错误。对不起，请重试".to_owned(),
            Self::NotImplemented => "😢 功能尚未实现，敬请期待".to_owned(),
        }
    }
}

/// 流程
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub enum State {
    #[default]
    Start,
    StartCb,
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
        obj_teacher: ObjTeacher,
    },
    Comment {
        object_id: String, // 待重构为 Obj
        comment_type: CommentType,
    },
    Publish {
        object_id: String, // 待重构为 Obj
        comment: String,
        comment_type: CommentType,
    },
}

/// 开始功能选择的回调
#[derive(Serialize, Deserialize, Debug)]
pub enum StartOp {
    Tree,           // 开始在树结构中定位
    FindSupervisor, // 快速查找教师
    FindComment,    // 快速查找评价
    Status,         // 统计与状态
                    // Find,   // 快速查找
}

/// 对象操作的回调
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
                serde_json::to_string(&ObjectOp::ReturnU).unwrap(),
            ),
            InlineKeyboardButton::callback(
                "↩️ 🏢",
                serde_json::to_string(&ObjectOp::ReturnD).unwrap(),
            ),
            InlineKeyboardButton::callback(
                "↩️ 👔",
                serde_json::to_string(&ObjectOp::ReturnS).unwrap(),
            ),
        ],
    ])
}

use teloxide::utils::markdown::escape;
/// 生成评价 markdown
pub fn get_comment_msg(
    object_id: &String,
    supervisor: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let coms = comments_msg_helper(object_id)?;
    let text = if !coms.is_empty() {
        format!(
            "{}\n_使用 /comment \\<id\\> 给评价写评价。_ ",
            coms.join("\n\n")
        )
    } else {
        "🈳 _此客体暂无评价！_".to_string()
    };
    let text = format!(
        "*👔 {} id: `{}` 的评价：*\n{}\n\
        \n\
        *请选择操作：*",
        escape(supervisor),
        &object_id,
        text
    );
    Ok(text)
}

fn comments_msg_helper(
    object_id: &String,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    SAFC_DB
        .find_comment(object_id)?
        .iter()
        .map(|c: &ObjComment| {
            Ok(format!(
                "💬 *data {} \\| from {} \\| id `{}`*\n\
                {}\n\
                {}\n",
                escape(c.date.as_str()),
                c.source_cate,
                c.id,
                escape(c.description.replace("<br>", "\n").as_str()),
                format_nested_comments(comments_msg_helper(&c.id)?)
            ))
        })
        .collect()
}

/// 格式化嵌套评价
fn format_nested_comments(comments: Vec<String>) -> String {
    if !comments.is_empty() {
        comments
            .iter()
            .map(|c| {
                c.lines()
                    .map(|l| format!(" \\| {}", l))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .collect::<Vec<_>>()
            .join(escape("---\n").as_str())
    } else {
        // " \\| _沙发虚位以待_".to_owned()
        escape(" ◻\n")
    }
}

#[test]
fn my_test() {
    println!("{}", serde_json::to_string(&ObjectOp::Read).unwrap());
    let msg = get_comment_msg(&"2ac4ae281b9a2528".to_string(), "谢洪涛").unwrap();
    println!("{}", msg);
}
