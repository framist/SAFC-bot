use safc::db::*;
use safc::sec::*;

// msg 是 bot 独用的 mod
mod msg;
use msg::*;

use teloxide::types::InputFile;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    types::{
        InlineKeyboardButton, InlineKeyboardMarkup, KeyboardButton, KeyboardMarkup, KeyboardRemove,
        ParseMode::MarkdownV2,
    },
    utils::command::BotCommands,
};

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
    #[command(description = "下载数据库")]
    DownloadDb,
    #[command(description = "评价")]
    Comment(String),
    #[command(description = "搜索")]
    Find(String),
    #[command(description = "删除评价")]
    Del(String),
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    log::info!("Starting SAFT bot for telegram ... by Framecraft");

    let bot = Bot::from_env();

    bot.set_my_commands(Command::bot_commands()) // 向 telegram 注册命令
        .await
        .expect("Failed to set bot commands to telegram");

    log::info!("Bot commands have been set");

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// 责任链模式 的 SAFC bot
/// branch 是分支的意思 参考：
/// https://docs.rs/dptree/0.3.0/dptree/prelude/struct.Handler.html#the-difference-between-chaining-and-branching
fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    // 命令
    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::Help].endpoint(help_command))
        .branch(case![Command::Cancel].endpoint(cancel_command))
        .branch(case![Command::Info].endpoint(info_command))
        .branch(case![Command::DownloadDb].endpoint(download_db_command))
        .branch(case![Command::Find(arg)].endpoint(find_command))
        .branch(case![Command::Comment(arg)].endpoint(comment_command))
        .branch(case![Command::Del(arg)].endpoint(delete_comment_command))
        .branch(dptree::endpoint(invalid_command));

    // 文本消息
    #[rustfmt::skip]
    let text_handler = Message::filter_text()
        .branch(case![State::FindSupervisor].endpoint(find_supervisor))
        .branch(case![State::FindComment].endpoint(find_comment))
        .branch(case![State::SchoolCate].endpoint(choose_university))
        .branch(case![State::University { school_cate }].endpoint(choose_department))
        .branch(case![State::Department { school_cate, university }].endpoint(choose_supervisor))
        .branch(case![State::Supervisor { school_cate, university, department }].endpoint(read_or_comment))
        .branch(case![State::Comment { object_id, comment_type }].endpoint(add_comment))
        .branch(case![State::Publish { object_id, comment, comment_type }].endpoint(publish_comment))
        .branch(case![State::DeleteConfirm { comment_id }].endpoint(confirm_delete_comment));

    // 消息
    let message_handler = Update::filter_message()
        .branch(command_handler) // 命令也是消息的一种
        .branch(text_handler)
        .branch(dptree::endpoint(invalid_state));

    // 回调
    // todo 解决回调信息的串扰问题
    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::StartCb].endpoint(start_cb))
        .branch(case![State::Read { obj_teacher }].endpoint(read_or_comment_cb))
        .branch(case![State::PagingCb { data }].endpoint(paging_cb))
        .branch(dptree::endpoint(invalid_callback_query));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
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

async fn download_db_command(bot: Bot, msg: Message) -> HandlerResult {
    let db_path = SAFC_DB.get_db_path();

    // 发送 "正在上传数据库..." 消息
    bot.send_message(msg.chat.id, "正在上传数据库...").await?;

    // 上传数据库文件
    let file = InputFile::file(db_path);
    match bot.send_document(msg.chat.id, file).await {
        Ok(_) => {
            bot.send_message(msg.chat.id, "数据库文件已成功上传。")
                .await?;
        }
        Err(err) => {
            bot.send_message(msg.chat.id, format!("上传数据库文件时出错：{}", err))
                .await?;
        }
    }

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
async fn find_command(bot: Bot, dialogue: MyDialogue, arg: String, msg: Message) -> HandlerResult {
    // arg 有效性验证
    let args: Vec<&str> = arg.split(' ').collect();
    if args.len() >= 2 {
        // 此处取下标必不会 panic
        match args[0] {
            "客体" => {
                find_supervisor_msg(&args[1..], &bot, &msg, dialogue).await?;
                return Ok(());
            }
            "评价" => {
                find_comment_msg(&args[1..], &bot, &msg, dialogue).await?;
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
            - /find 客体 赵__\n\
            - /find 评价 前途 无量\n\
            客体的一般是导师的意思。就可选的高级操作：\n\
            - 您可以用百分号（%）代表零个、一个或多个字符。下划线（_）代表一个单一的字符\n\n\
            您也可以使用 /start 中的功能按钮使用这些功能",
    )
    .await?;
    Ok(())
}

/// 删除评价命令
async fn delete_comment_command(
    bot: Bot,
    dialogue: MyDialogue,
    arg: String,
    msg: Message,
) -> HandlerResult {
    let parts: Vec<&str> = arg.split_whitespace().collect();

    if parts.len() != 2 {
        bot.send_message(
            msg.chat.id,
            "使用方法：\n/del <评价 ID> <发布人 OTP>\n示例：/del ab12cd34 xxxxxxxx",
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    let comment_id = parts[0].to_string();
    let otp = parts[1].to_string();

    let comment = match SAFC_DB.find_comment_by_id(&comment_id)? {
        Some(c) => c,
        None => {
            bot.send_message(
                msg.chat.id,
                format!("未找到评价 `{}`。请检查评价 ID。", comment_id),
            )
            .reply_to_message_id(msg.id)
            .await?;
            return Ok(());
        }
    };

    let author_sign = match comment.author_sign.as_ref() {
        Some(sign) => sign,
        None => {
            bot.send_message(
                msg.chat.id,
                "该评价未设置 OTP，暂不支持自行删除。",
            )
            .reply_to_message_id(msg.id)
            .await?;
            return Ok(());
        }
    };

    let expected_sign = hash_author_sign(&comment_id, &otp);
    if author_sign != &expected_sign {
        bot.send_message(
            msg.chat.id,
            "签名验证失败。请确认评价 ID 与 OTP 是否正确。",
        )
        .reply_to_message_id(msg.id)
        .await?;
        return Ok(());
    }

    let object_info = match SAFC_DB.find_objteacher_with_id(comment.object.as_str())? {
        Some(obj) => obj.display_path(),
        None => format!("客体 ID {}", comment.object),
    };

    let comment_body = escape(comment.description.as_str());
    let comment_type_label = comment.comment_type.to_string();
    let confirm_message = format!(
        "评价 ID `{}`\n对象：{}\n日期：{}\n类型：{}\n\n```text\n{}\n```\n\n签名验证成功。\n请回复「确认删除」以继续，或发送 /cancel 终止此操作。",
        escape(comment_id.as_str()),
        escape(object_info.as_str()),
        escape(comment.date.as_str()),
        escape(comment_type_label.as_str()),
        comment_body
    );

    bot.send_message(msg.chat.id, confirm_message)
        .reply_to_message_id(msg.id)
        .parse_mode(MarkdownV2)
        .await?;

    dialogue.update(State::DeleteConfirm { comment_id }).await?;
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

async fn invalid_state(_bot: Bot, msg: Message) -> HandlerResult {
    log::warn!("invalid state - Unable to handle the message: {:?}", msg);
    Ok(())
}

async fn invalid_command(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        format!("❌ 错误命令 - usage: \n{}", Command::descriptions()),
    )
    .await?;
    Ok(())
}

/// 开始
async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, TgResponse::Hello.to_string())
        .parse_mode(MarkdownV2)
        .reply_markup(start_op_keyboard())
        .reply_to_message_id(msg.id)
        .await?;
    dialogue.update(State::StartCb).await?; // 更新会话状态
    Ok(())
}

async fn start_cb(bot: Bot, dialogue: MyDialogue, q: CallbackQuery) -> HandlerResult {
    bot.answer_callback_query(q.id).await?; // todo 先别 await
    if let Some(op) = &q.data {
        match serde_json::from_str(op)? {
            StartOp::Tree => {
                let data = SAFC_DB.find_school_cate()?;
                let keyboard = convert_to_n_columns_keyboard(data, 3);
                let text = "您想查询或评价的「学校类别」是？您可以直接输入或者在下面的键盘选择框中选择\n\
                    _键盘选择框中没有的也可以直接输入来新建；如果是上个类别本身请选择或输入 `self`。下同_\n";
                bot.send_message(dialogue.chat_id(), text)
                    .parse_mode(MarkdownV2)
                    .reply_markup(KeyboardMarkup::new(keyboard))
                    .await?;
                dialogue.update(State::SchoolCate).await?; // 更新会话状态
            }
            StartOp::FindSupervisor => {
                let text = "请回复你要查找的 👔\n\
                    可选：您可以用百分号（%）代表零个、一个或多个字符。下划线（_）代表一个单一的字符\n\n\
                    例如：赵__\n\
                    此功能等效使用命令 /find 客体 赵__\n";
                bot.send_message(dialogue.chat_id(), text).await?;
                dialogue.update(State::FindSupervisor).await?;
            }
            StartOp::FindComment => {
                let text = "请回复你要查找的评价关键字，空格分隔\n\
                    可选：您可以用百分号（%）代表零个、一个或多个字符。下划线（_）代表一个单一的字符\n\n\
                    例如：前途 无量\n\
                    此功能等效使用命令 /find 评价 前途 无量"; // todo
                bot.send_message(dialogue.chat_id(), text).await?;
                dialogue.update(State::FindComment).await?;
            }
            StartOp::Status => {
                let text = SAFC_DB.db_status()?;
                bot.send_message(dialogue.chat_id(), text).await?;
            }
        }
    }
    Ok(())
}

/// 用于模糊搜索的关键字列表的处理函数
/// todo 去除顺序的影响
fn search_keys_helper(s: &[&str]) -> String {
    format!("%{}%", s.join("%"))
}

/// 快速查找客体的消息
/// 进入分页状态，最终的返回状态为 [`State::StartCb`]
async fn find_supervisor_msg(
    args: &[&str],
    bot: &Bot,
    msg: &Message,
    dialogue: MyDialogue,
) -> HandlerResult {
    let mut objs = SAFC_DB.find_supervisor_like(&search_keys_helper(args))?;
    if objs.is_empty() {
        bot.send_message(msg.chat.id, ":( 找不到所查询的导师")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(());
    }
    let action_name = "选定".to_string();
    objs.truncate(MSG_MAX_PAGES);
    let pages: Vec<String> = objs
        .clone()
        .into_iter()
        .map(|x| display_teacher_md(&x))
        .collect();
    let action_states = objs
        .clone()
        .into_iter()
        .map(|x| State::Read { obj_teacher: x })
        .collect();
    let action_msgs = objs
        .clone()
        .into_iter()
        .map(|x| format!("{}\n请选择操作：", x.display_path()))
        .collect();
    let text = &pages[0]; // assert!(pages.len() >= 1);
    bot.send_message(msg.chat.id, text)
        .reply_markup(build_paging_keyboard(pages.len(), 0, Some(&action_name)))
        .parse_mode(MarkdownV2)
        .reply_to_message_id(msg.id)
        .await?;
    dialogue
        .update(State::PagingCb {
            data: PagingCbData {
                pages,
                actions: Some(PagingCbActions {
                    name: action_name,
                    action_states,
                    action_msgs,
                    action_op_keyboard: obj_op_keyboard(),
                }),
                prev_state: Box::new(State::StartCb),
                prev_msg: "请选择操作：".to_string(),
                prev_op_keyboard: start_op_keyboard(),
            },
        })
        .await?;
    Ok(())
}

/// 快速查找评论的消息
/// 进入分页状态，最终的返回状态为 [`State::StartCb`]
async fn find_comment_msg(
    args: &[&str],
    bot: &Bot,
    msg: &Message,
    dialogue: MyDialogue,
) -> HandlerResult {
    let mut objs = SAFC_DB.find_comment_like(&search_keys_helper(args))?;
    if objs.is_empty() {
        bot.send_message(msg.chat.id, ":( 查询无结果")
            .reply_to_message_id(msg.id)
            .await?;
        return Ok(());
    }
    let action_name = "回复此评价".to_string();
    objs.truncate(MSG_MAX_PAGES);
    let pages: Vec<String> = objs
        .clone()
        .into_iter()
        .map(|c: ObjComment| {
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
        .collect();
    let action_states = objs
        .clone()
        .into_iter()
        .map(|c| State::Comment {
            object_id: c.id,
            comment_type: CommentType::Nest,
        })
        .collect();
    let action_msgs = objs
        .clone()
        .into_iter()
        .map(|c| {
            format!(
                "🆔 `{}`\n\
                \n请写下您对此客体的评价：",
                c.id
            )
        })
        .collect();
    let text = &pages[0]; // assert!(pages.len() >= 1);

    bot.send_message(msg.chat.id, text)
        .reply_markup(build_paging_keyboard(pages.len(), 0, Some(&action_name)))
        .parse_mode(MarkdownV2)
        .reply_to_message_id(msg.id)
        .await?;

    dialogue
        .update(State::PagingCb {
            data: PagingCbData {
                pages,
                actions: Some(PagingCbActions {
                    name: action_name,
                    action_states,
                    action_msgs,
                    ..Default::default()
                }),
                prev_state: Box::new(State::StartCb),
                prev_msg: "请选择操作：".to_string(),
                prev_op_keyboard: start_op_keyboard(),
            },
        })
        .await?;
    Ok(())
}

/// 快速查找教师 状态的处理函数
async fn find_supervisor(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let args: Vec<&str> = msg.text().ok_or("None msg err")?.split(' ').collect();
    find_supervisor_msg(&args, &bot, &msg, dialogue).await?;
    Ok(())
}

/// 快速查找教师 状态的处理函数
async fn find_comment(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    let args: Vec<&str> = msg.text().ok_or("None msg err")?.split(' ').collect();
    find_comment_msg(&args, &bot, &msg, dialogue).await?;
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
    let keyboard = convert_to_n_columns_keyboard(SAFC_DB.find_university(s_c)?, 2);
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
    // 获取学院列表并转换为单列键盘布局
    let keyboard = convert_to_n_columns_keyboard(SAFC_DB.find_department(s_c, university)?, 1);
    bot.send_message(
        msg.chat.id,
        format!("🧭 {s_c} 🏫 {university}\n您想查询的「学院」是："),
    )
    // 添加自定义键盘，增强用户交互
    .reply_markup(KeyboardMarkup::new(keyboard).input_field_placeholder("学院？".to_string()))
    // 将此消息设置为对原始消息的回复，增强对话的上下文关联
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
    let keyboard = convert_to_n_columns_keyboard(
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
                        "{}\n\
                        请选择操作：",
                        display_teacher_md(&obj_teacher)
                    ),
                )
                .reply_to_message_id(msg.id)
                .parse_mode(MarkdownV2)
                .reply_markup(obj_op_keyboard())
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
                let action_name = "回复此评价".to_string();
                let pages: Vec<String> = get_comment_pages(&object_id)?
                    .iter()
                    .enumerate()
                    .map(|(i, x)| {
                        format!(
                            "*👔 {} `{}` 的评价 第 {} 页：*\n\
                            {}\n\
                            _使用 /comment \\<id\\> 给任意评价写评价。_ ",
                            escape(&supervisor),
                            &object_id,
                            i + 1,
                            x
                        )
                    })
                    .collect();
                if pages.is_empty() {
                    if let Some(Message { id, chat, .. }) = q.message {
                        bot.edit_message_text(chat.id, id, "🈳 _此客体暂无评价！_".to_string())
                            .reply_markup(obj_op_keyboard())
                            .parse_mode(MarkdownV2)
                            .await?;
                    }
                    // dialogue.update(State::Read { obj_teacher }).await?; // 更新会话状态
                } else {
                    let text = &pages[0];
                    if let Some(Message { id, chat, .. }) = q.message {
                        bot.edit_message_text(chat.id, id, text)
                            .reply_markup(build_paging_keyboard(pages.len(), 0, Some(&action_name)))
                            .parse_mode(MarkdownV2)
                            .await?;
                    }
                    let comments = SAFC_DB.find_comment(&object_id)?;
                    let action_msgs = comments
                        .iter()
                        .map(|x| format!("回复评价 `{}`\n/cancel 取消", &x.id))
                        .collect();
                    let action_states = comments
                        .iter()
                        .map(|x| State::Comment {
                            object_id: x.id.clone(),
                            comment_type: CommentType::Nest,
                        })
                        .collect();

                    // 进入读评价的分页状态
                    dialogue
                        .update(State::PagingCb {
                            data: PagingCbData {
                                pages,
                                actions: Some(PagingCbActions {
                                    name: action_name,
                                    action_states,
                                    action_msgs,
                                    ..Default::default()
                                }),
                                prev_state: Box::new(State::Read { obj_teacher }),
                                prev_msg: escape(
                                    format!(
                                    "🧭 {school_cate} 🏫 {university} 🏢 {department} 👔 {supervisor}\n\
                                    请选择操作："
                                )
                                    .as_str(),
                                ),
                                prev_op_keyboard: obj_op_keyboard(),
                            },
                        })
                        .await?;
                }
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
                        .reply_markup(obj_op_keyboard())
                        .await?;
                } // else ... todo
                  // dialogue.update(State::Read { obj_teacher }).await?; // 更新会话状态
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
                // todo 转为使用 display_teacher_md
                let text = format!(
                    "🧭 {school_cate} 🏫 {university} 🏢 {department} 👔 {supervisor}\n\
                    该客体的初次添加日期：{}\n\
                    {:?} \n（此功能有待开发）",
                    date, info
                );
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, text)
                        .reply_markup(obj_op_keyboard())
                        .await?;
                } // else ... todo
                  // dialogue.update(State::Read { obj_teacher }).await?; // 更新会话状态
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

/// 分页会话回调处理函数
/// 注意，parse_mode(MarkdownV2) 现在还是必须的
/// 从回调中获取目前的页码 `index`
async fn paging_cb(
    bot: Bot,
    dialogue: MyDialogue,
    data: PagingCbData,
    q: CallbackQuery,
) -> HandlerResult {
    let PagingCbData {
        pages,
        actions,
        prev_state,
        prev_msg,
        prev_op_keyboard,
    } = data;
    bot.answer_callback_query(q.id).await?;
    if let Some(op) = &q.data {
        match serde_json::from_str(op)? {
            PagingOp::Page(index) => {
                let text = &pages[index];
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, text)
                        .parse_mode(MarkdownV2)
                        .reply_markup(build_paging_keyboard(
                            pages.len(),
                            index,
                            actions.map(|x| x.name.clone()).as_ref(),
                        ))
                        .await?;
                }
            }
            PagingOp::Back => {
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, prev_msg)
                        .parse_mode(MarkdownV2)
                        .reply_markup(prev_op_keyboard)
                        .await?;
                }
                dialogue.update(*prev_state).await?;
            }
            PagingOp::Action(index) => {
                if let Some(PagingCbActions {
                    name: _,
                    action_states,
                    action_msgs,
                    action_op_keyboard,
                }) = actions
                {
                    if let Some(Message { id, chat, .. }) = q.message {
                        bot.edit_message_text(chat.id, id, &action_msgs[index])
                            .parse_mode(MarkdownV2)
                            .reply_markup(action_op_keyboard)
                            .await?;
                    }
                    dialogue.update(action_states[index].clone()).await?;
                }
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
                "您对 `{}` 的评价是\n\n\
                ```\n{}\n```\n\
                确认发布？如确认请输入「发布人 OTP」，之后将发布评价;\
                取消请 /cancel  *您只能在此取消！*\n\
                _注：「发布人 OTP」即一次性密钥，是可以让您日后证明本评价由您发布，由此您可以修改/销毁此评论，\
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
                .reply_markup(obj_op_keyboard())
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

/// 删除评价确认处理
async fn confirm_delete_comment(
    bot: Bot,
    dialogue: MyDialogue,
    comment_id: String,
    msg: Message,
) -> HandlerResult {
    let text = match msg.text() {
        Some(t) => t.trim(),
        None => {
            bot.send_message(
                msg.chat.id,
                "未识别的输入。请回复「确认删除」，或使用 /cancel 取消操作。",
            )
            .await?;
            return Ok(());
        }
    };

    if text != "确认删除" {
        bot.send_message(
            msg.chat.id,
            "如需删除，请回复「确认删除」。若不再需要删除，可使用 /cancel 结束操作。",
        )
        .await?;
        return Ok(());
    }

    match SAFC_DB.delete_comment(&comment_id)? {
        true => {
            bot.send_message(
                msg.chat.id,
                format!("评价 `{}` 已删除。感谢您的更新！", comment_id),
            )
            .await?;
        }
        false => {
            bot.send_message(
                msg.chat.id,
                "评价不存在或已被删除。",
            )
            .await?;
        }
    }

    dialogue.update(State::Start).await?;
    Ok(())
}

/// 一维向量转换为 n 列纵向键盘
fn convert_to_n_columns_keyboard(data: Vec<String>, n: usize) -> Vec<Vec<KeyboardButton>> {
    data.chunks(n)
        .map(|chunk| chunk.iter().map(KeyboardButton::new).collect())
        .collect()
}
