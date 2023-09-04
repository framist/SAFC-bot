import os
import sqlite3
import logging
import hashlib
import datetime
from telegram import ReplyKeyboardMarkup, ReplyKeyboardRemove, InlineKeyboardButton, InlineKeyboardMarkup, Update
from typing import List
from telegram.constants import ParseMode
from telegram.ext import (
    Application,
    CommandHandler,
    CallbackQueryHandler,
    ContextTypes,
    ConversationHandler,
    MessageHandler,
    filters,
)


"""
demo bot 0.0
后续将采用 rust 重构
"""

# Enable logging
logging.basicConfig(
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s", level=logging.INFO
)
# set higher logging level for httpx to avoid all GET and POST requests being logged
logging.getLogger("httpx").setLevel(logging.WARNING)

logger = logging.getLogger(__name__)

# 主流程
SCHOOL_CATE, UNIVERSITY, DEPARTMENT, SUPERVISOR, COMMENT, PUBLISH = range(6)
# 选择好评价对象 object 后的菜单
OBJECT_READ, OBJECT_COMMENT, OBJECT_INFO, OBJECT_END, OBJECT_ADD = range(5)

BOT_INFO = """# 大学生反诈中心

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
* 只有评价，没有评分，每个客体都不能由单独的分数来决定

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

"""

BOT_HELP = """大学生反诈中心（SAFT）的机器人
/start - 开始
/cancel - 终止对话，无响应的时候就试试看这个吧
/help - 帮助
/info - 信息
"""

DATA_PATH = "./db.sqlite"

SAFC_ASLT = 'SAFC_salt'

# 纵向表格转换为 n 列纵向表格
def _convert_to_n_columns(data, n):
    data = [item[0] for item in data]
    return [data[i:i + n] for i in range(0, len(data), n)]

# 纵向表格转换为 3 列纵向表格
def _convert_to_3_columns(data):
    return _convert_to_n_columns(data, 3)


async def start(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    """Starts the conversation and asks the user about their school_cate."""
    context.user_data.clear()
    # TODO 用户限制
    # user = update.message.from_user

    with sqlite3.connect(DATA_PATH) as conn:
        cursor = conn.cursor()
        # 所有的学校类别
        cursor.execute("SELECT DISTINCT school_cate FROM objects")
        reply_keyboard = [list(item) for item in cursor.fetchall()]

    await update.message.reply_text(
        "嗨！我是大学生反诈中心的客服机器人 👋\n"
        "_目前仍为早期开发版本_ 问题敬请反馈；*越墙不易，延迟丢包敬请见谅，可/cancel /start 重启再试试*\n"
        "发送 /cancel 来停止此次对话\n\n"
        "您可以先查询客体，然后查看或发起对客体的评价。\n\n"
        "您想查询或评价的「学校类别」是？您可以直接输入或者在下面的键盘选择框中选择\n\n"
        "_键盘选择框中没有的也可以直接输入来新建；如果是上个类别本身请选择或输入 `self`。下同_\n"
        "（如果是在 PC 端群聊中使用，键盘选择框弹出可能有 bug）",
        reply_markup=ReplyKeyboardMarkup(
            _convert_to_3_columns(reply_keyboard), one_time_keyboard=True, input_field_placeholder="学校类别？"
        ),
        parse_mode=ParseMode.MARKDOWN_V2,
    )

    return SCHOOL_CATE


async def choose_university(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    """存储选定的 school_cate，并询问大学。"""
    s_c = update.message.text
    context.user_data['school_cate'] = s_c

    with sqlite3.connect(DATA_PATH) as conn:
        cursor = conn.cursor()
        # 指定学校类别下的学校
        cursor.execute("SELECT DISTINCT university FROM objects WHERE school_cate=?",
                       (s_c,))
        reply_keyboard = [list(item) for item in cursor.fetchall()]

    await update.message.reply_text(
        f"已选择：{context.user_data}\n"
        "您想查询的「学校」是？\n",
        reply_markup=ReplyKeyboardMarkup(
            reply_keyboard, one_time_keyboard=True, input_field_placeholder="学校？"
        )
    )

    return UNIVERSITY


async def choose_department(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    """Stores the selected university and asks for a department."""
    university = update.message.text
    context.user_data["university"] = university
    with sqlite3.connect(DATA_PATH) as conn:
        cursor = conn.cursor()
        # 指定学校下的学院
        cursor.execute("SELECT DISTINCT department FROM objects WHERE school_cate=? AND university=?",
                       (context.user_data['school_cate'], university))
        reply_keyboard = [list(item) for item in cursor.fetchall()]

    await update.message.reply_text(
        f"已选择：{context.user_data}\n"
        "您想查询的「学院」是？",
        reply_markup=ReplyKeyboardMarkup(
            reply_keyboard, one_time_keyboard=True, input_field_placeholder="学院？"
        ),
    )

    return DEPARTMENT


async def choose_supervisor(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    """Stores the selected department and asks for a supervisor."""
    department = update.message.text
    context.user_data["department"] = department
    with sqlite3.connect(DATA_PATH) as conn:
        cursor = conn.cursor()
        # 指定学院下的导师
        cursor.execute("SELECT DISTINCT supervisor FROM objects WHERE school_cate=? AND university=? AND department=?",
                       (context.user_data['school_cate'],
                        context.user_data["university"],
                        department))
        reply_keyboard = [list(item) for item in cursor.fetchall()]
    await update.message.reply_text(
        f"已选择：{context.user_data}\n"
        "您想查询的「导师或其他客体」是？",
        reply_markup=ReplyKeyboardMarkup(
            _convert_to_3_columns(reply_keyboard), one_time_keyboard=True, input_field_placeholder="导师？"
        ),
    )
    return SUPERVISOR


async def read_or_comment(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """存储选定的客体并询问下一步操作"""
    supervisor = update.message.text
    context.user_data["supervisor"] = supervisor
    with sqlite3.connect(DATA_PATH) as conn:
        cursor = conn.cursor()
        # objects
        cursor.execute("SELECT object FROM objects WHERE university=? AND department=? AND supervisor=?",
                       (context.user_data["university"],
                        context.user_data['department'],
                        context.user_data["supervisor"]))
        obj = cursor.fetchall()
    if obj:
        context.user_data["object_id"] = obj[0][0]
        await update.message.reply_text(
            f"已选择：{context.user_data}\n"
            f"请选择操作：",
            reply_markup=build_keyboard()
        )
    else:
        context.user_data["object_id"] = hashlib.sha256(f"{context.user_data['university']}{context.user_data['department']}{context.user_data['supervisor']}".encode()).hexdigest()[:16]
        await update.message.reply_text(
            f"已选择：{context.user_data}\n"
            f"🈳 目前还没有这个对象的信息，是否增加此对象？",
            reply_markup = InlineKeyboardMarkup([
                [
                    InlineKeyboardButton(text='➕ 增加', callback_data=str(OBJECT_ADD)),
                    InlineKeyboardButton(text='🏁 结束', callback_data=str(OBJECT_END)),
                ],
            ])
        )

def build_keyboard() -> InlineKeyboardMarkup:
    buttons = [
        [
            InlineKeyboardButton(text='👀 查看评价', callback_data=str(OBJECT_READ)),
            InlineKeyboardButton(text='➕ 增加评价', callback_data=str(OBJECT_COMMENT)),
        ],
        [
            InlineKeyboardButton(text='🤗 详细信息',callback_data=str(OBJECT_INFO)),
            InlineKeyboardButton(text='🏁 结束', callback_data=str(OBJECT_END)),
        ],
    ]
    return InlineKeyboardMarkup(buttons)


async def read_or_comment_cb(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    """Parses the CallbackQuery and updates the message text."""
    query = update.callback_query
    obj = context.user_data["object_id"]
    await query.answer()    # ? 为什么需要这行
    if query.data == str(OBJECT_READ):
        # 获取 object 的评价
        with sqlite3.connect(DATA_PATH) as conn:
            cursor = conn.cursor()
            cursor.execute("SELECT description, date, source_cate, id FROM comments WHERE object=? ",
                           (obj,))
            ans = [
                f"date: {item[1]} | from: {item[2]} | id: {item[3]}\n评价：\n{item[0]}"
                for item in cursor.fetchall()
            ]
            ans = "\n---\n".join(ans).replace("<br>", "\n") if ans else "🈳 此客体暂无评价！"

        await query.edit_message_text(
            f"已选择：{context.user_data}\n"
            f"此导师的评价是:\n\n{ans}",
            # "\n==========\n抱歉！对评价的评价暂不可用",
            # parse_mode = ParseMode.HTML
            reply_markup=build_keyboard()
        )
        return SUPERVISOR
    elif query.data == str(OBJECT_COMMENT):
        await query.edit_message_text(
            f"已选择：{context.user_data}\n"
            "【增加评价】\n为了您的隐私，请勿在群聊中使用！取消请 /cancel \n\n"
            "请写下您对此客体的评价："
        )
        return COMMENT
    elif query.data == str(OBJECT_END):
        await query.edit_message_text(
            "谢谢！本次对话结束。目前为测试版本，我们期待您的使用反馈",
        )
        return ConversationHandler.END
    elif query.data == str(OBJECT_ADD):
        # 增加评价客体
        with sqlite3.connect(DATA_PATH) as conn:
            cursor = conn.cursor()
            cursor.execute("""INSERT INTO objects 
                (school_cate, university, department, supervisor, date, object)
                VALUES (?, ?, ?, ?, ?, ?)""",
                (context.user_data['school_cate'], 
                 context.user_data['university'], 
                 context.user_data['department'], 
                 context.user_data['supervisor'], 
                 datetime.datetime.now().strftime("%Y-%m-%d"),
                 context.user_data["object_id"])
            )
            conn.commit()
        await query.edit_message_text(
            f"已选择：{context.user_data}\n"
            f"评价客体已增加！感谢您的贡献 🌷",
            reply_markup=build_keyboard()
        )
        logging.info("评价客体已增加！")
    else:
        await query.edit_message_text(
            f"已选择：{context.user_data}\n"
            f":( 抱歉本功能未开发",
            reply_markup=build_keyboard()
        )
        return SUPERVISOR


async def add_comment(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    """增加评价处理函数"""
    comment = update.message.text
    context.user_data["comment"] = comment
    date = datetime.datetime.now().strftime("%Y-%m-%d")
    context.user_data["date"] = date
    # 评价 id = sha256( object | 评价 | 日期 )[:16] 注意，这个也包含去重的性质
    comment_id = hashlib.sha256(
        f"{context.user_data['object_id']}{comment}{date}".encode()).hexdigest()[:16]
    context.user_data["comment_id"] = comment_id
    await update.message.reply_text(
        f"您的评价是```\n{comment}\n```\nid: {comment_id} | data: {date}\n"
        "确认发布？如确认请输入「发布人 OTP」，之后将发布评价;"
        "取消请 /cancel —— 您只能在此取消！\n"
        "Ps.「发布人 OTP」是可以让您日后证明本评价由您发布，由此您可以修改/销毁此评论，如不需要，输入随机值即可"
    )
    return PUBLISH


async def publish_comment(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    """增加评价处理函数"""
    s = update.message.text
    comment_id = context.user_data["comment_id"]
    # 发布人签名 = sha256( 评价 id | sha256(salt + 发布人 OTP) )
    sign = hashlib.sha256(f'{comment_id}'.encode() +
                          hashlib.sha256(f"{SAFC_ASLT}{s}".encode()).digest()).hexdigest()

    # 添加至数据库
    with sqlite3.connect(DATA_PATH) as conn:
        cursor = conn.cursor()
        cursor.execute("""INSERT INTO comments
            (object, description, date, source_cate, type, author_sign, id)
            VALUES (?, ?, ?, ?, ?, ?, ?)""",
                       (
                           context.user_data['object_id'],
                           context.user_data["comment"],
                           context.user_data['date'],
                           'telegram',
                           'teacher',  # TODO
                           sign,
                           comment_id
                       )
                       )

    await update.message.reply_text(
        f"您的 OTP 已销毁，生成签名 {sign}\n"
        "评价已发布！感谢您的贡献 🌷",
        reply_markup=build_keyboard()
    )
    logging.info("评价已发布！")
    return SUPERVISOR


async def cancel(update: Update, context: ContextTypes.DEFAULT_TYPE) -> int:
    context.user_data.clear()
    """Cancels and ends the conversation."""
    user = update.message.from_user
    logger.info("User %s canceled the conversation.", user.first_name)
    await update.message.reply_text(
        "您终止了本次会话\n再见！本次对话结束。我们期待您的使用反馈", reply_markup=ReplyKeyboardRemove()
    )

    return ConversationHandler.END


async def help_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Send a message when the command /help is issued."""
    await update.message.reply_text(
        BOT_HELP
    )


async def info_command(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    """Send a message when the command /info is issued."""
    await update.message.reply_text(
        BOT_INFO,
        # parse_mode=ParseMode.MARKDOWN_V2
    )


def main() -> None:
    """Run the bot."""
    TOKEN = os.getenv("TELOXIDE_TOKEN")

    # 创建应用程序并将其传递给您的机器人的令牌。
    application = Application.builder().token(TOKEN).build()

    # 流程控制
    conv_handler = ConversationHandler(
        entry_points=[CommandHandler("start", start)],
        states={
            SCHOOL_CATE: [MessageHandler(filters.TEXT & ~filters.COMMAND, choose_university)],
            UNIVERSITY: [MessageHandler(filters.TEXT & ~filters.COMMAND, choose_department)],
            DEPARTMENT: [MessageHandler(filters.TEXT & ~filters.COMMAND, choose_supervisor)],
            SUPERVISOR: [MessageHandler(filters.TEXT & ~filters.COMMAND, read_or_comment),
                         CallbackQueryHandler(read_or_comment_cb)],
            # READ: [MessageHandler(filters.TEXT & ~filters.COMMAND, read_comment)],
            COMMENT: [MessageHandler(filters.TEXT & ~filters.COMMAND, add_comment)],
            PUBLISH: [MessageHandler(filters.TEXT & ~filters.COMMAND, publish_comment)],
        },
        fallbacks=[CommandHandler("cancel", cancel)]
    )

    application.add_handler(conv_handler)
    application.add_handler(CommandHandler("help", help_command))
    application.add_handler(CommandHandler("info", info_command))

    # Run the bot until the user presses Ctrl-C
    application.run_polling(allowed_updates=Update.ALL_TYPES)


if __name__ == "__main__":
    main()
