use safc::db::*;
// use safc::msg::*;
use safc::sec::*;
mod msg;
use msg::*;

use teloxide::types::ParseMode::MarkdownV2;
use teloxide::utils::markdown::escape;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup, KeyboardRemove,
    },
    utils::command::BotCommands,
};

use url::Url;

type MyDialogue = Dialogue<State, InMemStorage<State>>; // ? 要使用 sqlite 存储状态吗
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(BotCommands, Clone, PartialEq, Debug)]
#[command(
    rename_rule = "lowercase",
    // parse_with = "split",
    description = "这是大学生反诈中心（SAFC @SAFC_group）的机器人\n支持以下命令："
)]
enum Command {
    #[command(description = "显示帮助信息")]
    Help,
    #[command(description = "开始")]
    Start,
    #[command(description = "终止对话")]
    Cancel,
    #[command(description = "信息")]
    Info,
    #[command(description = "评价")]
    Comment(String),
    #[command(description = "搜索")]
    Find(String),
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting SAFT bot...\nby Framecraft");

    let bot = Bot::from_env();

    bot.set_my_commands(Command::bot_commands()) // 向 telegram 注册命令
        .await
        .expect("Failed to set bot commands to telegram");

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// 责任链模式
/// branch 是分支的意思 参考：
/// https://docs.rs/dptree/0.3.0/dptree/prelude/struct.Handler.html#the-difference-between-chaining-and-branching
fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    // 命令
    let command_handler = teloxide::filter_command::<Command, _>()
        // .branch(
        //     case![State::Start].branch(case![Command::Start].endpoint(start)), // 只有 start 状态下才能用 /start
        // )
        .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::Help].endpoint(help_command))
        .branch(case![Command::Cancel].endpoint(cancel_command))
        .branch(case![Command::Info].endpoint(info_command))
        .branch(case![Command::Find(arg)].endpoint(find_command))
        .branch(case![Command::Comment(arg)].endpoint(comment_command))
        .branch(dptree::endpoint(invalid_command));

    // 消息
    let message_handler = Update::filter_message()
        .branch(command_handler) // 命令也是消息的一种
        .branch(case![State::SchoolCate].endpoint(choose_university))
        .branch(case![State::University { school_cate }].endpoint(choose_department))
        .branch(
            case![State::Department {
                school_cate,
                university
            }]
            .endpoint(choose_supervisor),
        )
        .branch(
            case![State::Supervisor {
                school_cate,
                university,
                department
            }]
            .endpoint(read_or_comment),
        )
        .branch(
            case![State::Comment {
                object_id,
                comment_type
            }]
            .endpoint(add_comment),
        )
        .branch(
            case![State::Publish {
                object_id,
                comment,
                comment_type
            }]
            .endpoint(publish_comment),
        )
        .branch(dptree::endpoint(invalid_state));

    // 回调
    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::StartCb].endpoint(start_cb))
        .branch(case![State::Read { obj_teacher }].endpoint(read_or_comment_cb))
        .branch(dptree::endpoint(invalid_callback_query));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        // .branch(Message::filter_text().branch(message_handler)) // TODO
        .branch(message_handler)
        .branch(callback_query_handler)
}

/// Send a message when the command /help is issued.
async fn help_command(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .reply_to_message_id(msg.id)
        .await?;
    Ok(())
}

/// Send a message when the command /info is issued.
async fn info_command(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, TgResponse::Info.to_string())
        .reply_to_message_id(msg.id)
        .parse_mode(MarkdownV2)
        .await?;
    Ok(())
}

/// Cancels and ends the conversation.
async fn cancel_command(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "您终止了本次会话\n再见！本次对话结束，使用 /start 重新开始。\n我们期待您的使用反馈",
    )
    .reply_to_message_id(msg.id)
    .reply_markup(KeyboardRemove::new())
    .await?;

    dialogue.exit().await?;
    Ok(())
}

/// find_command 快速查找
/// todo 改为回调的形式，来支持翻页，查找功能选择等问题
async fn find_command(bot: Bot, _dialogue: MyDialogue, arg: String, msg: Message) -> HandlerResult {
    let j = |x: &[&str]| format!("%{}%", x.join("%"));
    // arg 有效性验证
    let args: Vec<&str> = arg.split(' ').collect();
    if args.len() >= 2 {
        match args[0] {
            "客体" => {
                let text = SAFC_DB
                    .find_supervisor_like(&j(&args[1..]))?
                    .into_iter()
                    .map(|x| x.join(" > "))
                    .collect::<Vec<String>>();
                let text = if text.len() > 20 {
                    format!("条目过多，仅显示前 20 条\n{}", text[..20].join("\n"))
                // todo 应能翻页来显示所有
                } else {
                    text.join("\n")
                };
                bot.send_message(msg.chat.id, text)
                    .reply_to_message_id(msg.id)
                    .await?;
                return Ok(());
            }
            "评价" => {
                let text = SAFC_DB
                    .find_comment_like(&j(&args[1..]))?
                    .iter()
                    .map(|c: &ObjComment| {
                        format!(
                            "💬 *针对 object `{}` 的评价：*\n\
                            *data {} \\| from {} \\| id `{}`*\n\
                            {}\n",
                            c.object,
                            escape(c.date.as_str()),
                            c.source_cate,
                            c.id,
                            escape(c.description.replace("<br>", "\n").as_str())
                        )
                    })
                    .collect::<Vec<String>>();
                let text = if text.len() > 5 {
                    format!("_条目过多，仅显示前 5 条_\n{}", text[..5].join("\n"))
                // todo 应能翻页来显示所有
                } else {
                    text.join("\n")
                };

                bot.send_message(msg.chat.id, text)
                    .reply_to_message_id(msg.id)
                    .parse_mode(MarkdownV2)
                    .await?;
                return Ok(());
            }
            _ => {}
        }
    }
    bot.send_message(
        msg.chat.id,
        "使用方法： \n\
            - /find <客体 | 评价> <关键字 1> [关键字...]\n\
            例如：\n\
            - /find 客体 习__\n\
            - /find 评价 前途 无量\n\
            可选的高级操作：\n\
            - 您可以用百分号（%）代表零个、一个或多个字符。下划线（_）代表一个单一的字符\n\n\
            目前的此命令操作是临时的，后续会改为内联按钮的形式来支持翻页，功能选择等",
    )
    .await?;
    Ok(())
}

/// 直接评价命令处理函数
async fn comment_command(
    bot: Bot,
    dialogue: MyDialogue,
    arg: String,
    msg: Message,
) -> HandlerResult {
    // arg 有效性验证
    if arg.is_empty() {
        bot.send_message(msg.chat.id, "使用方法： /comment <id>")
            .await?;
        return Ok(());
    }

    if let Some(t) = SAFC_DB.if_object_exists(&arg)? {
        let object_id = arg;

        let text = format!(
            "🆔 `{object_id}`\n\
            \n请写下您对此客体的评价："
        );

        bot.send_message(msg.chat.id, text)
            .reply_to_message_id(msg.id)
            .parse_mode(MarkdownV2)
            .reply_markup(KeyboardRemove::new())
            .await?;

        dialogue
            .update(State::Comment {
                object_id,
                comment_type: t,
            })
            .await?; // 更新会话状态
        Ok(())
    } else {
        bot.send_message(msg.chat.id, "❌ - 非有效 id").await?;
        Ok(())
    }
}

async fn _unable_command(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, TgResponse::NotImplemented.to_string())
        .await?;
    Ok(())
}

async fn invalid_state(_bot: Bot, _msg: Message) -> HandlerResult {
    // bot.send_message(msg.chat.id, "❎ 错误流程 - Type /help to see the usage.")
    //     .await?;
    log::warn!("invalid_state - Unable to handle the message.");
    Ok(())
}

async fn invalid_command(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        format!("❎ 错误命令 - usage: \n{}", Command::descriptions()),
    )
    .await?;
    log::warn!("invalid_command - Unable to handle the command");
    Ok(())
}

/// old 开始对话，并向用户询问他们的 school_cate。
async fn _start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    // let data = find_school_cate()?;
    let data = SAFC_DB.find_school_cate()?;
    let keyboard = _convert_to_n_columns_keyboard(data, 3);
    bot.send_message(msg.chat.id, TgResponse::Hello.to_string())
        .parse_mode(MarkdownV2)
        .reply_markup(KeyboardMarkup::new(keyboard))
        .await?;
    dialogue.update(State::SchoolCate).await?; // 更新会话状态
    Ok(())
}

/// 开始
async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, TgResponse::Hello.to_string())
        .parse_mode(MarkdownV2)
        .reply_markup(InlineKeyboardMarkup::new([
            vec![InlineKeyboardButton::callback(
                "🌳 开始查询&评价！",
                serde_json::to_string(&StartOp::Tree).unwrap(),
            )],
            vec![
                InlineKeyboardButton::callback(
                    "👔 快搜教师",
                    serde_json::to_string(&StartOp::FindSupervisor).unwrap(),
                ),
                InlineKeyboardButton::callback(
                    "💬 快搜评论",
                    serde_json::to_string(&StartOp::FindComment).unwrap(),
                ),
            ],
            vec![
                InlineKeyboardButton::callback(
                    "📊",
                    serde_json::to_string(&StartOp::Status).unwrap(),
                ),
                InlineKeyboardButton::url("🏛️", Url::parse("https://t.me/SAFC_group").unwrap()),
                // InlineKeyboardButton::url("🌐", Url::parse("https://").unwrap()),
                InlineKeyboardButton::url("🐱", Url::parse(GITHUB_URL).unwrap()),
            ],
        ]))
        .reply_to_message_id(msg.id)
        .await?;
    dialogue.update(State::StartCb).await?; // 更新会话状态
    Ok(())
}

async fn start_cb(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    bot.answer_callback_query(q.id).await?;
    if let Some(op) = &q.data {
        match serde_json::from_str(op)? {
            StartOp::Tree => {
                let data = SAFC_DB.find_school_cate()?;
                let keyboard = _convert_to_n_columns_keyboard(data, 3);
                let text = "您想查询或评价的「学校类别」是？您可以直接输入或者在下面的键盘选择框中选择\n\
                    _键盘选择框中没有的也可以直接输入来新建；如果是上个类别本身请选择或输入 `self`。下同_\n";
                bot.send_message(dialogue.chat_id(), text)
                    .parse_mode(MarkdownV2)
                    .reply_markup(KeyboardMarkup::new(keyboard))
                    .await?;
                dialogue.update(State::SchoolCate).await?; // 更新会话状态
            }
            StartOp::FindSupervisor => {
                // let text = "请回复你要查找的 👔\n\
                // 可选：您可以用百分号（%）代表零个、一个或多个字符。下划线（_）代表一个单一的字符\n\n\
                // 例如：习__\n\
                // 也可以使用命令 /find 客体 习__\n";
                let text = "功能尚未实现\n请使用命令 /find";
                bot.send_message(dialogue.chat_id(), text).await?;
            }
            StartOp::FindComment => {
                let text = "功能尚未实现\n请使用命令 /find";
                bot.send_message(dialogue.chat_id(), text).await?;
            }
            StartOp::Status => {
                let text = SAFC_DB.db_status()?;
                bot.send_message(dialogue.chat_id(), text).await?;
            }
        }
    }
    Ok(())
}

/// 存储选定的 school_cate，并询问 university。
async fn choose_university(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    if let Some(s_c) = msg.text().map(ToOwned::to_owned) {
        choose_university_msg(&s_c, &bot, &msg).await?;
        dialogue
            .update(State::University { school_cate: s_c })
            .await?; // 更新会话状态
    } else {
        bot.send_message(msg.chat.id, TgResponse::RetryErrNone.to_string())
            .await?;
    }

    Ok(())
}

async fn choose_university_msg(s_c: &String, bot: &Bot, msg: &Message) -> HandlerResult {
    let keyboard = _convert_to_n_columns_keyboard(SAFC_DB.find_university(s_c)?, 2);
    bot.send_message(msg.chat.id, format!("🧭 {s_c}\n您想查询的「学校」是："))
        .reply_markup(KeyboardMarkup::new(keyboard).input_field_placeholder("学校？".to_string()))
        .reply_to_message_id(msg.id)
        .await?;
    Ok(())
}

/// 存储选定的 university 并要求一个 department。
async fn choose_department(
    bot: Bot,
    dialogue: MyDialogue,
    s_c: String, // Available from `State::...`.
    msg: Message,
) -> HandlerResult {
    if let Some(university) = msg.text().map(ToOwned::to_owned) {
        choose_department_msg(&s_c, &university, &bot, &msg).await?;
        dialogue
            .update(State::Department {
                school_cate: s_c,
                university,
            })
            .await?; // 更新会话状态
    } else {
        bot.send_message(msg.chat.id, TgResponse::RetryErrNone.to_string())
            .await?;
    }
    Ok(())
}

async fn choose_department_msg(
    s_c: &String,
    university: &String,
    bot: &Bot,
    msg: &Message,
) -> HandlerResult {
    let keyboard = _convert_to_n_columns_keyboard(SAFC_DB.find_department(s_c, university)?, 1);
    bot.send_message(
        msg.chat.id,
        format!("🧭 {s_c} 🏫 {university}\n您想查询的「学院」是："),
    )
    .reply_markup(KeyboardMarkup::new(keyboard).input_field_placeholder("学院？".to_string()))
    .reply_to_message_id(msg.id)
    .await?;
    Ok(())
}

/// 存储所选部门并选择 客体
async fn choose_supervisor(
    bot: Bot,
    dialogue: MyDialogue,
    (s_c, university): (String, String), // Available from `State::...`.
    msg: Message,
) -> HandlerResult {
    if let Some(department) = msg.text().map(ToOwned::to_owned) {
        choose_supervisor_msg(&s_c, &university, &department, &bot, &msg).await?;
        dialogue
            .update(State::Supervisor {
                school_cate: s_c,
                university,
                department,
            })
            .await?; // 更新会话状态
    } else {
        bot.send_message(msg.chat.id, TgResponse::RetryErrNone.to_string())
            .await?;
    }
    Ok(())
}

async fn choose_supervisor_msg(
    school_cate: &String,
    university: &String,
    department: &String,
    bot: &Bot,
    msg: &Message,
) -> HandlerResult {
    let keyboard = _convert_to_n_columns_keyboard(
        SAFC_DB.find_supervisor(school_cate, university, department)?,
        3,
    );
    bot.send_message(
        msg.chat.id,
        format!("🧭 {school_cate} 🏫 {university} 🏢 {department}\n您想查询的「导师等客体」是："),
    )
    .reply_markup(KeyboardMarkup::new(keyboard).input_field_placeholder("客体？".to_string()))
    .reply_to_message_id(msg.id)
    .await?;
    Ok(())
}

/// 存储选定的客体并询问下一步操作
async fn read_or_comment(
    bot: Bot,
    dialogue: MyDialogue,
    (school_cate, university, department): (String, String, String), // Available from `State::...`.
    msg: Message,
) -> HandlerResult {
    if let Some(supervisor) = msg.text().map(ToOwned::to_owned) {
        let obj = SAFC_DB.find_object_with_path(&university, &department, &supervisor)?;
        match obj {
            None => {
                let object_id = hash_object_id(&university, &department, &supervisor);
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "🧭 {school_cate} 🏫 {university} 🏢 {department} 👔 {supervisor}\n\
                        🤗 目前还没有这个对象的信息，是否增加此对象？"
                    ),
                )
                .reply_markup(InlineKeyboardMarkup::new([[
                    InlineKeyboardButton::callback(
                        "➕ 增加",
                        serde_json::to_string(&ObjectOp::Add).unwrap(),
                    ),
                    InlineKeyboardButton::callback(
                        "🏁 结束",
                        serde_json::to_string(&ObjectOp::End).unwrap(),
                    ),
                ]]))
                .reply_to_message_id(msg.id)
                .await?;
                dialogue
                    .update(State::Read {
                        obj_teacher: ObjTeacher {
                            school_cate,
                            university,
                            department,
                            supervisor,
                            date: get_current_date(),
                            info: None,
                            object_id,
                        },
                    })
                    .await?; // 更新会话状态
            }
            Some(obj_teacher) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "🧭 {school_cate} 🏫 {university} 🏢 {department} 👔 {supervisor}\n\
                        请选择操作："
                    ),
                )
                .reply_to_message_id(msg.id)
                .reply_markup(build_op_keyboard())
                .await?;
                dialogue.update(State::Read { obj_teacher }).await?; // 更新会话状态
            }
        }
    } else {
        bot.send_message(msg.chat.id, TgResponse::RetryErrNone.to_string())
            .await?;
    }
    Ok(())
}

/// 解析 CallbackQuery 并更新消息文本。
async fn read_or_comment_cb(
    bot: Bot,
    dialogue: MyDialogue,
    obj_teacher: ObjTeacher, // Available from `State::...`.
    q: CallbackQuery,
) -> HandlerResult {
    let ObjTeacher {
        school_cate,
        university,
        department,
        supervisor,
        date,
        info,
        object_id,
    } = obj_teacher.clone();
    bot.answer_callback_query(q.id).await?;
    if let Some(op) = &q.data {
        match serde_json::from_str(op)? {
            ObjectOp::Read => {
                let text = get_comment_msg(&object_id, &supervisor)?;
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, text)
                        .reply_markup(build_op_keyboard())
                        .parse_mode(MarkdownV2)
                        .await?;
                }
                // else if let Some(id) = q.inline_message_id {
                //     bot.edit_message_text_inline(id, text).await?; // 使用户自己发言的情况（inline 模式）todo
                // } else {
                //     log::error!("unhanded q.message");
                // }
                dialogue.update(State::Read { obj_teacher }).await?; // 更新会话状态
            }
            ObjectOp::Add => {
                // 增加评价客体
                SAFC_DB.add_object(&obj_teacher)?;
                let text = format!(
                    "🧭 {school_cate} 🏫 {university} 🏢 {department} 👔 {supervisor}\n\
                    评价客体已增加！感谢您的贡献 🌷"
                );
                log::info!("评价客体已增加！");
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, text)
                        .reply_markup(build_op_keyboard())
                        .await?;
                } // else ... todo
                dialogue.update(State::Read { obj_teacher }).await?; // 更新会话状态
            }
            ObjectOp::Commet => {
                let text = format!(
                    "🧭 {school_cate} 🏫 {university} 🏢 {department} 👔 {supervisor}\n\
                    \n请写下您对此客体的评价："
                );
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, text).await?;
                } // else ... todo
                dialogue
                    .update(State::Comment {
                        object_id,
                        comment_type: CommentType::Teacher,
                    })
                    .await?; // 更新会话状态
            }
            ObjectOp::End => {
                bot.send_message(
                    dialogue.chat_id(),
                    "谢谢！本次对话结束，使用 /start 重新开始。\n目前为测试版本，我们期待您的使用反馈".to_string(),
                )
                .reply_markup(KeyboardRemove::new())
                .await?;
                dialogue.exit().await?; // 结束会话
            }
            ObjectOp::Info => {
                let text = format!(
                    "🧭 {school_cate} 🏫 {university} 🏢 {department} 👔 {supervisor}\n\
                    该客体的初次添加日期：{}\n\
                    wiki：{:?} （此功能有待开发）",
                    date, info
                );
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, text)
                        .reply_markup(build_op_keyboard())
                        .await?;
                } // else ... todo
                dialogue.update(State::Read { obj_teacher }).await?; // 更新会话状态
            }
            ObjectOp::ReturnU => {
                choose_university_msg(&school_cate, &bot, &q.message.unwrap()).await?;
                dialogue.update(State::University { school_cate }).await?;
            }
            ObjectOp::ReturnD => {
                choose_department_msg(&school_cate, &university, &bot, &q.message.unwrap()).await?;
                dialogue
                    .update(State::Department {
                        school_cate,
                        university,
                    })
                    .await?;
            }
            ObjectOp::ReturnS => {
                choose_supervisor_msg(
                    &school_cate,
                    &university,
                    &department,
                    &bot,
                    &q.message.unwrap(),
                )
                .await?;
                dialogue
                    .update(State::Supervisor {
                        school_cate,
                        university,
                        department,
                    })
                    .await?;
            }
        }
    }

    Ok(())
}

async fn invalid_callback_query(bot: Bot, q: CallbackQuery) -> HandlerResult {
    bot.answer_callback_query(q.id).await?;
    if let Some(Message { id, chat, .. }) = q.message {
        bot.edit_message_text(chat.id, id, "❎ 对话过期。使用 /start 重新开始")
            .await?;
    }
    Ok(())
}

/// 增加评价处理函数
/// ? 返回字符串使用的标记语言是什么
async fn add_comment(
    bot: Bot,
    dialogue: MyDialogue,
    (object_id, comment_type): (String, CommentType), // Available from `State::...`.
    msg: Message,
) -> HandlerResult {
    if let Some(comment) = msg.text().map(ToOwned::to_owned) {
        // let date = get_current_date();
        // let comment_id = hash_comment_id(&object_id, &comment, &date);
        bot.send_message(
            msg.chat.id,
            format!(
                "您对 `{}` 的评价是\n\
                ```\n{}\n```\n\
                确认发布？如确认请输入「发布人 OTP」，之后将发布评价;\
                取消请 /cancel  *您只能在此取消！*\n\
                _注：「发布人 OTP」是可以让您日后证明本评价由您发布，由此您可以修改/销毁此评论，\
                如不需要，输入随机值即可_",
                &object_id,
                escape(comment.as_str())
            ),
        )
        .reply_to_message_id(msg.id)
        .parse_mode(MarkdownV2)
        .await?;
        dialogue
            .update(State::Publish {
                object_id,
                comment,
                comment_type,
            })
            .await?; // 更新会话状态
    } else {
        bot.send_message(msg.chat.id, TgResponse::RetryErrNone.to_string())
            .await?;
    }
    Ok(())
}

/// 增加评价处理函数
async fn publish_comment(
    bot: Bot,
    dialogue: MyDialogue,
    (object_id, comment, comment_type): (String, String, CommentType), // Available from `State::...`.
    msg: Message,
) -> HandlerResult {
    if let Some(otp) = msg.text().map(ToOwned::to_owned) {
        let c = ObjComment::new_with_otp(
            object_id.clone(),
            comment,
            SourceCate::Telegram,
            comment_type,
            otp,
        );
        SAFC_DB.add_comment(&c)?; // ? 有些可能的错误需提示用户
        log::info!("{} 评价已发布", c.id);

        match SAFC_DB.find_objteacher_with_id(object_id.as_str())? {
            Some(obj_teacher) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "_您的 OTP 已销毁_\n\
                        评价「`{}`」已发布！感谢您的贡献 🌷",
                        c.id
                    ),
                )
                .reply_to_message_id(msg.id)
                .parse_mode(MarkdownV2)
                .reply_markup(build_op_keyboard())
                .await?;
                dialogue.update(State::Read { obj_teacher }).await?;
            }
            None => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "_您的 OTP 已销毁_\n\
                        嵌套评价「`{}`」已发布！感谢您的贡献 🌷\n\
                        使用 /start 重新开始",
                        c.id
                    ),
                )
                .reply_to_message_id(msg.id)
                .parse_mode(MarkdownV2)
                .await?;
                dialogue.exit().await?; // TODO 嵌套评价面板
            }
        }
    } else {
        bot.send_message(msg.chat.id, TgResponse::RetryErrNone.to_string())
            .await?;
    }
    Ok(())
}

/// 一维向量转换为 n 列纵向键盘
fn _convert_to_n_columns_keyboard(data: Vec<String>, n: usize) -> Vec<Vec<KeyboardButton>> {
    data.chunks(n)
        .map(|chunk| chunk.iter().map(KeyboardButton::new).collect())
        .collect()
}
