//! # db
//!
//! 数据、数据库有关操作
//!
//! 原始数据的处理、建立数据库脚本文件。请保留以供未来参考
//!
//! 使用 sqlite - 关系型数据库，弱类型
//!
//! ```
//! 【客体表】objects
//! _学校类别 < 学校 < 学院 < 导师 - _日期 - _信息 - object (key)
//!           | 包含学院本身 self 下同
//!
//! object：
//! sha256( 学校 | 学院 | 导师 )[:8]
//!
//! 【评价表】comments
//! object < 评价 - 日期 - _来源分类 - _评价类型 - 发布人签名 - 评价 id (key)
//!
//! `_` 表示后续可变
//! 来源分类：admin, urfire, telegram...
//! 评价类型：nest（评价的评价）, teacher, course, student, unity, info（wiki_like） ...
//! 评价 id = sha256( object | 评价 | 日期 )[:8] 注意，这个也包含去重的性质
//! 发布人签名 可为空 = sha256( 评价 id | sha256(salt + 发布人一次性密语).hex )
//! salt: SAFC_salt
//! ```
//!
//! TODO 转换为严格的关系型数据库，目前为了敏捷开发，使用 ~2NF
//!
//! TODO 备份与发布
//!
//! TODO 区块链、分布式数据库？- 基于 telegram 通讯
//!

use crate::sec::*;
use rusqlite::{params, Connection, Result};
use std::str::FromStr;
use strum_macros::{Display, EnumString};

use chrono;

/// 来源分类：admin, urfire, telegram...
#[derive(Debug, EnumString, Display, PartialEq)] // ?
#[strum(serialize_all = "lowercase")]
pub enum SourceCate {
    Admin,
    Urfire,
    Telegram,
    Web,
}

pub struct Object {
    pub school_cate: SourceCate,
    pub university: String,
    pub department: String,
    pub supervisor: String,
    pub date: String,
    pub info: Option<String>,
    pub object: String,
}

use serde::{Deserialize, Serialize};

/// 评价类型：nest（评价的评价）, teacher, course, student, unity, info（wiki_like）
#[derive(Debug, EnumString, Display, PartialEq, Clone, Deserialize, Serialize)]
#[strum(serialize_all = "lowercase")]
pub enum CommentType {
    Nest,
    Teacher,
    Course,
    Student,
    Unity,
    Info,
}

#[derive(Debug)]
pub struct Comment {
    pub object: String,
    pub description: String,
    pub date: String,
    pub source_cate: SourceCate,
    pub comment_type: CommentType,
    pub author_sign: Option<String>,
    pub id: String,
}

use std::env;
fn db_open() -> Result<Connection> {
    // Read environment variable and set DB_PATH
    let db_path = match env::var("SAFC_DB_PATH") {
        Ok(val) => val,
        Err(_) => {
            log::warn!("SAFC_DB_PATH not set, set to default: ./db.sqlite");
            "db.sqlite".to_string()
        }
    };

    Connection::open(db_path)
}

pub fn find_school_cate() -> Result<Vec<String>> {
    let conn = db_open()?;

    let mut stmt = conn.prepare("SELECT DISTINCT school_cate FROM objects")?;
    let rows = stmt.query_map([], |row| row.get::<usize, String>(0))?;

    rows.collect::<Result<Vec<_>, _>>()
}

pub fn find_university(s_c: &String) -> Result<Vec<String>> {
    let conn = db_open()?;

    let mut stmt =
        conn.prepare("SELECT DISTINCT university FROM objects WHERE school_cate=(?1)")?;
    let rows = stmt.query_map([s_c], |row| row.get(0))?;

    rows.collect::<Result<Vec<_>, _>>()
}

pub fn find_department(s_c: &String, university: &String) -> Result<Vec<String>> {
    let conn = db_open()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT department FROM objects WHERE \
        school_cate=(?1) AND university=(?2)",
    )?;
    let rows = stmt.query_map([s_c, university], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>()
}

pub fn find_supervisor(
    s_c: &String,
    university: &String,
    department: &String,
) -> Result<Vec<String>> {
    let conn = db_open()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT supervisor FROM objects WHERE \
        school_cate=(?1) AND university=(?2)  AND department=(?3)",
    )?;
    let rows = stmt.query_map([s_c, university, department], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>()
}

/// 模糊搜索
/// 百分号（%）代表零个、一个或多个字符。下划线（_）代表一个单一的字符。这些符号可以被组合使用。
pub fn find_supervisor_like(s: &String) -> Result<Vec<Vec<String>>> {
    let conn = db_open()?;

    let mut stmt = conn.prepare(
        "SELECT school_cate, university, department, supervisor, object FROM objects WHERE \
        supervisor LIKE (?1)",
    )?;
    // let rows = stmt.query_map([s], |row| row.get(0))?;
    let rows = stmt.query_map([s], |row| (0..5).map(|i| row.get(i)).collect())?;
    rows.collect::<Result<Vec<_>, _>>()
}

#[test]
fn test_find_object() {
    let s = &"习__".to_string();
    let result = find_supervisor_like(s);
    println!("{:#?}", result);
}

pub fn find_object(
    university: &String,
    department: &String,
    supervisor: &String,
) -> Result<Vec<String>> {
    let conn = db_open()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT object FROM objects WHERE \
        supervisor=(?1) AND university=(?2) AND department=(?3)",
    )?;

    let rows = stmt.query_map([supervisor, university, department], |row| {
        row.get::<_, String>(0)
    })?;
    rows.collect::<Result<Vec<_>, _>>()
}

/// object 是否存在
/// TODO 重构：数据库加入客体的种类
pub fn if_object_exists(object: &str) -> Result<Option<CommentType>> {
    let conn = db_open()?;

    let exists1: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM objects WHERE object = ?1)",
        [object],
        |row| row.get(0),
    )?;

    if exists1 {
        return Ok(Some(CommentType::Teacher));
    }

    let exists2: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM comments WHERE id = ?1)",
        [object],
        |row| row.get(0),
    )?;

    if exists2 {
        return Ok(Some(CommentType::Nest));
    }

    Ok(None)
}

#[test]
fn test_if_object_exists() {
    let result = if_object_exists("835cc322b7691485");
    println!("{:#?}", result);
}

/// 查找发布人
///
/// 【发布人表】publishers : object < 发布人 - 姓名 - 学号 - 邮箱 - 签名 - 头像
/// - object TEXT NOT NULL,
/// - name TEXT NOT NULL,
/// - student_id TEXT NOT NULL,
/// - email TEXT NOT NULL,
/// - signature TEXT,
/// - avatar TEXT,

/// 查找评价
///
/// 【评价表】comments : object < 评价 - 日期 - _来源分类 - _评价类型 - 发布人签名 - 评价 id (key)
/// - object TEXT NOT NULL,
/// - description TEXT NOT NULL,
/// - date TEXT NOT NULL,
/// - source_cate TEXT NOT NULL,
/// - type TEXT NOT NULL,
/// - author_sign TEXT,
/// - id TEXT NOT NULL,
pub fn find_comment(object_id: &String) -> Result<Vec<Comment>> {
    let conn = db_open()?;

    let mut stmt = conn.prepare("SELECT * FROM comments WHERE object=? ")?;
    let rows = stmt.query_map([object_id], |row| {
        Ok(Comment {
            object: row.get::<_, String>(0)?,
            description: row.get::<_, String>(1)?,
            date: row.get::<_, String>(2)?,
            source_cate: SourceCate::from_str(row.get::<_, String>(3)?.as_str()).unwrap(),
            comment_type: CommentType::from_str(row.get::<_, String>(4)?.as_str()).unwrap(),
            author_sign: match row.get::<_, String>(5) {
                Ok(s) => Some(s),
                Err(_) => None,
            },
            id: row.get::<_, String>(6)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
}

pub fn find_comment_like(s: &String) -> Result<Vec<Comment>> {
    let conn = db_open()?;

    let mut stmt = conn.prepare("SELECT * FROM comments WHERE description LIKE ? ")?;
    let rows = stmt.query_map([s], |row| {
        Ok(Comment {
            object: row.get::<_, String>(0)?,
            description: row.get::<_, String>(1)?,
            date: row.get::<_, String>(2)?,
            source_cate: SourceCate::from_str(row.get::<_, String>(3)?.as_str()).unwrap(),
            comment_type: CommentType::from_str(row.get::<_, String>(4)?.as_str()).unwrap(),
            author_sign: match row.get::<_, String>(5) {
                Ok(s) => Some(s),
                Err(_) => None,
            },
            id: row.get::<_, String>(6)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
}

#[test]
fn test_find_comment_like() {
    let comments = find_comment_like(&"%习大大%".to_string());
    println!("{:#?}", comments);
}

/// 增加评价客体，有一些值在函数内计算
pub fn add_object_to_db(
    // conn: &Connection,
    school_cate: &String,
    university: &String,
    department: &String,
    supervisor: &String,
    data: &String,
) -> Result<(), rusqlite::Error> {
    let conn = db_open()?;
    let object_id = hash_object_id(university, department, supervisor);

    conn.execute(
        "INSERT INTO objects (school_cate, university, department, supervisor, date, object) 
        VALUES (?, ?, ?, ?, ?, ?)",
        params![
            school_cate,
            university,
            department,
            supervisor,
            data,
            object_id
        ],
    )?;

    Ok(())
}

pub fn add_comment_to_db(
    // conn: &Connection,
    object_id: &String,
    comment: &String,
    date: &String,
    source_cate: SourceCate,
    comment_type: &String,
    otp: &String,
) -> Result<(), rusqlite::Error> {
    let conn = db_open()?;
    let comment_id = hash_comment_id(object_id, comment, date);
    let sign = hash_author_sign(&comment_id, otp);
    conn.execute(
        "INSERT INTO comments
        (object, description, date, source_cate, type, author_sign, id)
        VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            object_id,
            comment,
            date,
            source_cate.to_string(),
            comment_type,
            sign,
            comment_id
        ],
    )?;

    Ok(())
}

/// 统计数据库的信息
/// 总条目数，最近一月新增的条目数...
pub fn db_status() -> Result<String, rusqlite::Error> {
    let conn = db_open()?;

    let c_count =
        conn.query_row::<i32, _, _>("SELECT COUNT(*) FROM comments", [], |row| row.get(0))?;

    let o_count =
        conn.query_row::<i32, _, _>("SELECT COUNT(*) FROM objects", [], |row| row.get(0))?;

    let start = chrono::Local::now() - chrono::Duration::days(31);
    let m_count = conn.query_row::<i32, _, _>(
        "SELECT COUNT(*) FROM comments WHERE date > ?",
        [start.format("%Y-%m-%d").to_string()],
        |row| row.get(0),
    )?;

    Ok(format!(
        "评价总数：{}, 客体总数：{}, 月新增评价数：{}",
        c_count, o_count, m_count
    ))
}

#[test]
fn db_status_test() {
    println!("{:#?}", db_status());
}

pub fn get_current_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[test]
fn my_test() {
    println!("{:#?}", find_school_cate().unwrap());
    println!("{:#?}", find_university(&"985".to_string()).unwrap());
    println!(
        "{:#?}",
        find_department(&"985".to_string(), &"清华大学".to_string()).unwrap()
    );
    // println!(
    //     "{}",
    //     comments_msg_md(&"b148b44fd82fda41".to_string()).unwrap()[0]
    // );
}

#[test]
fn my_test2() {
    assert_eq!("admin".to_owned(), SourceCate::Admin.to_string());
    assert_eq!("nest".to_owned(), CommentType::Nest.to_string());
}
