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
//! TODO 参考：
//! https://course.rs/advance/errors.html - 归一化不同的错误类型
//!

use crate::sec::*;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum_macros::{Display, EnumString};

use chrono;

use r2d2_sqlite::{self, SqliteConnectionManager};

type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
type HandlerResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
// type HandlerResult<T, E = Box<dyn std::error::Error + Send + Sync>> > = std::result::Result<T, E>;

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

pub struct SAFCdb {
    pool: Pool,
}

impl Default for SAFCdb {
    fn default() -> Self {
        Self::new()
    }
}

impl SAFCdb {
    pub fn new() -> Self {
        let db_path = std::env::var("SAFC_DB_PATH").unwrap_or_else(|_| {
            log::warn!("SAFC_DB_PATH 未设置，默认设置为：./db.sqlite");
            "db.sqlite".to_string()
        });

        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::new(manager).unwrap();
        SAFCdb { pool }
    }

    pub fn find_school_cate(&self) -> HandlerResult<Vec<String>> {
        let conn = self.pool.clone().get()?;
        let mut stmt = conn.prepare("SELECT DISTINCT school_cate FROM objects")?;
        let rows = stmt.query_map([], |row| row.get::<usize, String>(0))?;

        // rows.collect::<Result<Vec<_>, _>>() // ? 可以将错误进行隐式的强制转换
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn find_university(&self, s_c: &String) -> HandlerResult<Vec<String>> {
        let conn = self.pool.clone().get()?;

        let mut stmt =
            conn.prepare("SELECT DISTINCT university FROM objects WHERE school_cate=(?1)")?;
        let rows = stmt.query_map([s_c], |row| row.get(0))?;

        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn find_department(&self, s_c: &String, university: &String) -> HandlerResult<Vec<String>> {
        let conn = self.pool.clone().get()?;

        let mut stmt = conn.prepare(
            "SELECT DISTINCT department FROM objects WHERE \
        school_cate=(?1) AND university=(?2)",
        )?;
        let rows = stmt.query_map([s_c, university], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn find_supervisor(
        &self,
        s_c: &String,
        university: &String,
        department: &String,
    ) -> HandlerResult<Vec<String>> {
        let conn = self.pool.clone().get()?;

        let mut stmt = conn.prepare(
            "SELECT DISTINCT supervisor FROM objects WHERE \
        school_cate=(?1) AND university=(?2)  AND department=(?3)",
        )?;
        let rows = stmt.query_map([s_c, university, department], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 模糊搜索
    /// 百分号（%）代表零个、一个或多个字符。下划线（_）代表一个单一的字符。这些符号可以被组合使用。
    pub fn find_supervisor_like(&self, s: &String) -> HandlerResult<Vec<Vec<String>>> {
        let conn = self.pool.clone().get()?;

        let mut stmt = conn.prepare(
            "SELECT school_cate, university, department, supervisor, object FROM objects WHERE \
        supervisor LIKE (?1)",
        )?;
        // let rows = stmt.query_map([s], |row| row.get(0))?;
        let rows = stmt.query_map([s], |row| (0..5).map(|i| row.get(i)).collect())?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn find_object(
        &self,
        university: &String,
        department: &String,
        supervisor: &String,
    ) -> HandlerResult<Vec<String>> {
        let conn = self.pool.clone().get()?;

        let mut stmt = conn.prepare(
            "SELECT DISTINCT object FROM objects WHERE \
        supervisor=(?1) AND university=(?2) AND department=(?3)",
        )?;

        let rows = stmt.query_map([supervisor, university, department], |row| {
            row.get::<_, String>(0)
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// object 是否存在
    /// TODO 重构：数据库加入客体的种类
    pub fn if_object_exists(&self, object: &str) -> HandlerResult<Option<CommentType>> {
        let conn = self.pool.clone().get()?;

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
    pub fn find_comment(&self, object_id: &String) -> HandlerResult<Vec<Comment>> {
        let conn = self.pool.clone().get()?;

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
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn find_comment_like(&self, s: &String) -> HandlerResult<Vec<Comment>> {
        let conn = self.pool.clone().get()?;

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
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 增加评价客体，有一些值在函数内计算
    pub fn add_object_to_db(
        &self,
        // conn: &Connection,
        school_cate: &String,
        university: &String,
        department: &String,
        supervisor: &String,
        data: &String,
    ) -> HandlerResult<()> {
        let conn = self.pool.clone().get()?;
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
        &self,
        object_id: &String,
        comment: &String,
        date: &String,
        source_cate: SourceCate,
        comment_type: &String,
        otp: &String,
    ) -> HandlerResult<()> {
        let conn = self.pool.clone().get()?;
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
    pub fn db_status(&self) -> HandlerResult<String> {
        let conn = self.pool.clone().get()?;

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
}

pub fn get_current_date() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

#[test]
fn test_find_object() {
    let db = SAFCdb::new();
    let s = &"习__".to_string();
    let result = db.find_supervisor_like(s);
    println!("{:#?}", result);
}

#[test]
fn test_if_object_exists() {
    let db = SAFCdb::new();
    let result = db.if_object_exists("835cc322b7691485");
    println!("{:#?}", result);
}

#[test]
fn test_find_comment_like() {
    let db = SAFCdb::new();
    let comments = db.find_comment_like(&"%习大大%".to_string());
    println!("{:#?}", comments);
}

#[test]
fn db_status_test() {
    let db = SAFCdb::new();
    println!("{:#?}", db.db_status());
}

#[test]
fn my_test() {
    let db = SAFCdb::new();
    println!("{:#?}", db.find_school_cate().unwrap());
    println!("{:#?}", db.find_university(&"985".to_string()).unwrap());
    println!(
        "{:#?}",
        db.find_department(&"985".to_string(), &"清华大学".to_string())
            .unwrap()
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
