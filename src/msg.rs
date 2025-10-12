use serde::{Deserialize, Serialize};
use teloxide::types::InlineKeyboardButton;
use teloxide::types::InlineKeyboardMarkup;
pub use teloxide::utils::markdown::escape;
use url::Url;

use safc::db::*;

// 有没有更优雅的方法？
// 放在这里是为了让 `main` 引用，而不是 `msg` 引用 `main`
use lazy_static::lazy_static;
lazy_static! {
    /// bot 的全局唯一数据库池
    pub static ref SAFC_DB: SAFCdb = SAFCdb::new();
}

/// 最大分页大小
pub const MSG_MAX_PAGES: usize = 99;

const GITHUB_URL: &str = "https://github.com/framist/SAFC-bot";
const WEB_URL: &str = "https://safc-web.vercel.app/";
const BOT_INFO: &str = r"*大学生反诈中心*

_元平台，分布式_

自从最初的导师评价网（urfire）关闭，时至今日，一批一批的新导师评价数据分享平台的迭起兴衰，最终都落于 404 或收费闭塞。
不知是何等阻力，让受过欺骗的学生和亟需信息的学生散若渺茫星火。故建此平台与机器人，革新方式，坚持“社群，保护，开放”的理念，信奉||密码朋克||、开源精神，愿此和谐共赢地持久性发展传承下去。

*telegram 机器人* ~@SAFC\_bot~ @SAFC\_bak\_bot —— 学校、专业、学院、课程、导师的交叉评价与查询
*telegram 群组社区* @SAFC\_group —— 公告与交流平台

[Web 端使用](https://safc-web.vercel.app/) 
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
                "👋 嗨！我是大学生反诈中心（SAFC @SAFC\\_group）的客服机器人\n",
                "_目前仍为早期开发版本_ 问题敬请反馈；*越墙不易，延迟丢包敬请见谅*\n",
                "您可以发送 /cancel 来停止此次对话；",
                "发送 /info 了解反诈中心\n",
                "您可以先查询客体（一般来讲客体就是导师啦），然后查看或发起对客体的评价。\n\n",
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
    /// 开始功能选单回调状态
    StartCb,
    /// 快速搜索教师
    FindSupervisor,
    /// 快速搜索评论
    FindComment,
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
        object_id: String, // todo 待重构为 Obj
        comment_type: CommentType,
    },
    Publish {
        object_id: String, // todo 待重构为 Obj
        comment: String,
        comment_type: CommentType,
    },
    DeleteConfirm {
        comment_id: String,
    },
    /// 分页显示回调状态
    PagingCb {
        data: PagingCbData,
    },
}

/// 分页显示回调状态的数据
#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct PagingCbData {
    /// 各个页面的文字
    pub pages: Vec<String>,
    /// 可选的对各页的进一步操作
    pub actions: Option<PagingCbActions>,
    /// 上一个状态
    pub prev_state: Box<State>,
    /// 用于返回后的消息显示
    /// msg 一定是 Markdown 格式的
    pub prev_msg: String,
    /// 用于返回后的内联回调键盘
    pub prev_op_keyboard: InlineKeyboardMarkup,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct PagingCbActions {
    /// 操作名，简短！
    pub name: String,
    /// 如操作，各个页面的下一个状态
    pub action_states: Vec<State>,
    /// 如操作，各个页面的下一个消息显示
    /// msg 一定是 Markdown 格式的
    pub action_msgs: Vec<String>,
    /// 如操作，页面统一的可选的内联回调键盘
    /// 若不需要，则 default 即可
    pub action_op_keyboard: InlineKeyboardMarkup,
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

/// 分页操作的回调
#[derive(Serialize, Deserialize, Debug)]
pub enum PagingOp {
    /// 页码
    Page(usize),
    /// 返回
    Back,
    /// 对当页对象的操作
    Action(usize),
}

impl From<StartOp> for String {
    fn from(val: StartOp) -> Self {
        serde_json::to_string(&val).unwrap()
    }
}

impl From<ObjectOp> for String {
    fn from(val: ObjectOp) -> Self {
        serde_json::to_string(&val).unwrap()
    }
}

impl From<PagingOp> for String {
    fn from(val: PagingOp) -> Self {
        serde_json::to_string(&val).unwrap()
    }
}

impl From<String> for ObjectOp {
    fn from(value: String) -> Self {
        serde_json::from_str(&value).unwrap()
    }
}

pub fn start_op_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![InlineKeyboardButton::callback(
            "🌳 开始查询 & 评价（荐）",
            StartOp::Tree,
        )],
        vec![
            InlineKeyboardButton::callback("👔 快搜教师", StartOp::FindSupervisor),
            InlineKeyboardButton::callback("💬 快搜评论", StartOp::FindComment),
        ],
        vec![
            InlineKeyboardButton::callback("📊", StartOp::Status),
            InlineKeyboardButton::url("🏛️", Url::parse("https://t.me/SAFC_group").unwrap()),
            InlineKeyboardButton::url("🌐", Url::parse(WEB_URL).unwrap()),
            InlineKeyboardButton::url("🐱", Url::parse(GITHUB_URL).unwrap()),
        ],
    ])
}

pub fn obj_op_keyboard() -> InlineKeyboardMarkup {
    InlineKeyboardMarkup::new([
        vec![
            InlineKeyboardButton::callback("👀 查看评价", ObjectOp::Read),
            InlineKeyboardButton::callback("💬 增加评价", ObjectOp::Commet),
        ],
        vec![
            InlineKeyboardButton::callback("🤗 详细信息", ObjectOp::Info),
            InlineKeyboardButton::callback("🏁 结束会话", ObjectOp::End),
        ],
        vec![
            InlineKeyboardButton::callback("↩️ 🏫", ObjectOp::ReturnU),
            InlineKeyboardButton::callback("↩️ 🏢", ObjectOp::ReturnD),
            InlineKeyboardButton::callback("↩️ 👔", ObjectOp::ReturnS),
        ],
    ])
}

/// `index` 从 0 开始的页码
/// `total` 为总共的页数
/// `action` 用于当前页的回调按钮
pub fn build_paging_keyboard(
    total: usize,
    index: usize,
    action: Option<&String>,
) -> InlineKeyboardMarkup {
    let mut buttons_2 = vec![InlineKeyboardButton::callback("↩️ 返回", PagingOp::Back)];
    if total <= 1 && action.is_none() {
        return InlineKeyboardMarkup::new([buttons_2]);
    }

    const COLS: usize = 2; // COLS * 2 + 1 == 一行显示最多的页码按钮数
    let start = index
        .saturating_sub(COLS)
        .min(total.saturating_sub(2 * COLS + 1));
    let mut buttons_1: Vec<InlineKeyboardButton> = (start..(start + 2 * COLS + 1).min(total))
        .map(|x| InlineKeyboardButton::callback(format!("{}/{}", x + 1, total), PagingOp::Page(x)))
        .collect();

    // buttons_1.retain(|x| x.text != format!("{}/{}", index + 1, total)); // 弃用
    for x in buttons_1.iter_mut() {
        if x.text == format!("{}/{}", index + 1, total) {
            *x = match action {
                Some(b) => InlineKeyboardButton::callback(b.clone(), PagingOp::Action(index)),
                None => InlineKeyboardButton::callback(
                    format!(">{}<", index + 1),
                    PagingOp::Action(index),
                ),
            };
            break;
        }
    }

    if index > 0 {
        buttons_2.push(InlineKeyboardButton::callback(
            "⬅️ 上页",
            PagingOp::Page(index - 1),
        ));
    }
    if index < total - 1 {
        buttons_2.push(InlineKeyboardButton::callback(
            "➡️ 下页",
            PagingOp::Page(index + 1),
        ));
    }
    InlineKeyboardMarkup::new([buttons_1, buttons_2])
}

/// 显示 [`ObjTeacher`] 详细的信息
/// markdown 格式
pub fn display_teacher_md(obj: &ObjTeacher) -> String {
    format!(
        "*{}*\n\
        信息：{}\n\
        评价数： {}\n\
        该客体的初次添加日期：{}",
        escape(obj.display_path().as_str()),
        escape(obj.info.clone().unwrap_or("暂无".to_string()).as_str()),
        escape(
            SAFC_DB
                .find_comment(&obj.object_id)
                .unwrap()
                .len()
                .to_string()
                .as_str()
        ),
        escape(obj.date.as_str())
    )
}

/// 生成分页的评价 markdown
pub fn get_comment_pages(
    object_id: &String,
) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    comments_msg_helper(object_id)
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
    let msg = get_comment_pages(&"2ac4ae281b9a2528".to_string()).unwrap();
    println!("{:#?}", msg);
}
