//! # db
//!
//! 数据、数据库有关操作
//!
//! 原始数据的处理、建立数据库脚本文件。请保留以供未来参考
//!
//! 使用 sqlite - 关系型数据库，弱类型
//!
//! 【客体表】objects
//! _学校类别 < _学校 < _学院 < _导师 - _日期 - _信息 - object (key)
//!           | 包含学院本身 self 下同
//! - school_cate TEXT NOT NULL,
//! - university TEXT NOT NULL,
//! - department TEXT NOT NULL,
//! - supervisor TEXT NOT NULL,
//! - date TEXT NOT NULL,
//! - info TEXT,
//! - object TEXT NOT NULL,
//! - PRIMARY KEY (object)
//! object：仅在第一次添加客体时计算，所以其他字段也可是可变的
//! sha256( 学校 | 学院 | 导师 )[:8byte]
//!
//! 【评价表】comments
//! object < 评价 - 日期 - _来源分类 - _评价类型 - 发布人签名 - 评价 id (key)
//! - object TEXT NOT NULL,
//! - description TEXT NOT NULL,
//! - date TEXT NOT NULL,
//! - source_cate TEXT NOT NULL,
//! - type TEXT NOT NULL,
//! - author_sign TEXT,
//! - id TEXT NOT NULL,
//!
//! `_` 表示后续可变
//! 来源分类：admin, urfire, telegram...
//! 评价类型：nest（评价的评价）, teacher, course, student, unity, info（wiki_like） ...
//! 评价 id = sha256( object | 评价 | 日期 )[:8byte] 注意，这个也包含去重的性质
//! 发布人签名 可为空 = sha256( 评价 id | sha256(salt + 发布人一次性密语).hex )
//! salt: SAFC_salt
//!
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

pub type Pool = r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>;
type HandlerResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
// type HandlerResult<T, E = Box<dyn std::error::Error + Send + Sync>> > = std::result::Result<T, E>;

/// 数据来源分类
#[derive(Debug, EnumString, Display, PartialEq, Clone, Serialize, Deserialize)]
#[strum(serialize_all = "lowercase")]
pub enum SourceCate {
    /// 管理员手动添加
    Admin,
    /// 来自 Urfire 数据
    /// 数据来源：https://gitee.com/wdwdwd123/RateMySupervisor
    /// 这个仓库可能 fork from https://github.com/pengp25/RateMySupervisor 且更新一点
    Urfire,
    /// 来自 SAFC 的 telegram 平台数据
    Telegram,
    /// 来自 SAFC 的 web 平台数据（暂无，平台未提供）
    Web,
    /// 来自 pi-review.com 数据 截止至 2023-09-26
    PiReview,
}

pub enum Obj {
    /// 导师...类客体
    Object(ObjTeacher),
    /// 评论类客体
    Comment(ObjComment),
}

/// 对应数据库中的【客体表（主要是导师）】objects
/// 只是 teacher-like，客体表的 object，不一定只是指导师
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ObjTeacher {
    pub school_cate: String,
    pub university: String,
    pub department: String,
    pub supervisor: String,
    pub date: String,
    pub info: Option<String>,
    pub object_id: String, // 此客体的 id
}

impl ObjTeacher {
    pub fn display_path(&self) -> String {
        format!(
            "🧭 {} 🏫 {} 🏢 {} 👔 {}",
            self.school_cate, self.university, self.department, self.supervisor
        )
    }
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

/// 对应数据库中的【评价表】comments
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ObjComment {
    pub object: String,
    pub description: String,
    pub date: String,
    pub source_cate: SourceCate,
    pub comment_type: CommentType,
    pub author_sign: Option<String>,
    pub id: String,
}

impl ObjComment {
    pub fn new_with_otp(
        object_id: String,
        comment: String,
        source_cate: SourceCate,
        comment_type: CommentType,
        otp: String,
    ) -> Self {
        let date = get_current_date();
        let id = hash_comment_id(&object_id, &comment, &date);
        let author_sign = Some(hash_author_sign(&id, &otp));
        ObjComment {
            object: object_id,
            description: comment,
            date,
            source_cate,
            comment_type,
            author_sign,
            id,
        }
    }
}

pub struct SAFCdb {
    pool: Pool,
}

impl Clone for SAFCdb {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
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

        Self::new_with_path(db_path)
    }

    pub fn new_with_path(db_path: String) -> Self {
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
    ///
    /// 返回搜到的 [`ObjTeacher`] 列表，可为空
    pub fn find_supervisor_like(&self, s: &String) -> HandlerResult<Vec<ObjTeacher>> {
        let conn = self.pool.clone().get()?;

        let mut stmt = conn.prepare(
            "SELECT * FROM objects WHERE \
            supervisor LIKE (?1)",
        )?;
        // let rows = stmt.query_map([s], |row| row.get(0))?;
        let rows = stmt.query_map([s], |row| {
            Ok(ObjTeacher {
                school_cate: row.get::<_, String>(0)?,
                university: row.get::<_, String>(1)?,
                department: row.get::<_, String>(2)?,
                supervisor: row.get::<_, String>(3)?,
                date: row.get::<_, String>(4)?,
                info: row.get::<_, String>(5).ok(),
                object_id: row.get::<_, String>(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 查找客体 用路径的方式
    /// 【客体表】objects  _学校类别 < 学校 < 学院 < 导师 - _日期 - _信息 - object (key)
    /// - school_cate TEXT NOT NULL,
    /// - university TEXT NOT NULL,
    /// - department TEXT NOT NULL,
    /// - supervisor TEXT NOT NULL,
    /// - date TEXT NOT NULL,
    /// - info TEXT,
    /// - object TEXT NOT NULL,
    /// - PRIMARY KEY (object)
    pub fn find_object_with_path(
        &self,
        university: &String,
        department: &String,
        supervisor: &String,
    ) -> HandlerResult<Option<ObjTeacher>> {
        let conn = self.pool.clone().get()?;

        let mut stmt = conn.prepare(
            "SELECT * FROM objects WHERE \
        supervisor=(?1) AND university=(?2) AND department=(?3)",
        )?;

        let rows = stmt.query_map([supervisor, university, department], |row| {
            Ok(ObjTeacher {
                school_cate: row.get::<_, String>(0)?,
                university: row.get::<_, String>(1)?,
                department: row.get::<_, String>(2)?,
                supervisor: row.get::<_, String>(3)?,
                date: row.get::<_, String>(4)?,
                info: row.get::<_, String>(5).ok(),
                object_id: row.get::<_, String>(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?.first().cloned())
    }

    /// 查找客体 用 id 的方式
    /// 【客体表】objects  _学校类别 < 学校 < 学院 < 导师 - _日期 - _信息 - object (key)
    pub fn find_objteacher_with_id(&self, object_id: &str) -> HandlerResult<Option<ObjTeacher>> {
        let conn = self.pool.clone().get()?;

        let mut stmt = conn.prepare("SELECT * FROM objects WHERE object=(?1)")?;

        let rows = stmt.query_map([object_id], |row| {
            Ok(ObjTeacher {
                school_cate: row.get::<_, String>(0)?,
                university: row.get::<_, String>(1)?,
                department: row.get::<_, String>(2)?,
                supervisor: row.get::<_, String>(3)?,
                date: row.get::<_, String>(4)?,
                info: row.get::<_, String>(5).ok(),
                object_id: row.get::<_, String>(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?.first().cloned())
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

    /// 通过评价对象 `object_id` 查找评价
    ///
    /// 【评价表】comments : object < 评价 - 日期 - _来源分类 - _评价类型 - 发布人签名 - 评价 id (key)
    /// - object TEXT NOT NULL,
    /// - description TEXT NOT NULL,
    /// - date TEXT NOT NULL,
    /// - source_cate TEXT NOT NULL,
    /// - type TEXT NOT NULL,
    /// - author_sign TEXT,
    /// - id TEXT NOT NULL,
    ///
    /// 返回 [`ObjComment`]
    pub fn find_comment(&self, object_id: &String) -> HandlerResult<Vec<ObjComment>> {
        let conn = self.pool.clone().get()?;

        let mut stmt = conn.prepare("SELECT * FROM comments WHERE object=? ")?;
        let rows = stmt.query_map([object_id], |row| {
            Ok(ObjComment {
                object: row.get::<_, String>(0)?,
                description: row.get::<_, String>(1)?,
                date: row.get::<_, String>(2)?,
                source_cate: SourceCate::from_str(row.get::<_, String>(3)?.as_str()).unwrap(),
                comment_type: CommentType::from_str(row.get::<_, String>(4)?.as_str()).unwrap(),
                author_sign: row.get::<_, String>(5).ok(),
                id: row.get::<_, String>(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// 查找评价 - like 方式
    pub fn find_comment_like(&self, s: &String) -> HandlerResult<Vec<ObjComment>> {
        let conn = self.pool.clone().get()?;

        let mut stmt = conn.prepare("SELECT * FROM comments WHERE description LIKE ? ")?;
        let rows = stmt.query_map([s], |row| {
            Ok(ObjComment {
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
    pub fn add_object(&self, obj_teacher: &ObjTeacher) -> HandlerResult<()> {
        let conn = self.pool.clone().get()?;
        conn.execute(
            "INSERT INTO objects (school_cate, university, department, supervisor, date, info, object) 
        VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                obj_teacher.school_cate,
                obj_teacher.university,
                obj_teacher.department,
                obj_teacher.supervisor,
                obj_teacher.date,
                obj_teacher.info,
                obj_teacher.object_id
            ],
        )?;

        Ok(())
    }

    /// 增加评价
    pub fn add_comment(&self, obj_comment: &ObjComment) -> HandlerResult<()> {
        let conn = self.pool.clone().get()?;
        conn.execute(
            "INSERT INTO comments
        (object, description, date, source_cate, type, author_sign, id)
        VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                obj_comment.object,
                obj_comment.description,
                obj_comment.date,
                obj_comment.source_cate.to_string(),
                obj_comment.comment_type.to_string(),
                obj_comment.author_sign,
                obj_comment.id
            ],
        )?;

        Ok(())
    }

    /// 统计数据库的信息
    /// 总条目数，最近一月新增的条目数...
    pub fn db_status(&self) -> HandlerResult<String> {
        let conn = self.pool.clone().get()?;

        let o_count =
            conn.query_row::<i32, _, _>("SELECT COUNT(*) FROM objects", [], |row| row.get(0))?;

        let c_count =
            conn.query_row::<i32, _, _>("SELECT COUNT(*) FROM comments", [], |row| row.get(0))?;

        let start = chrono::Local::now() - chrono::Duration::days(31);
        let m_new = conn.query_row::<i32, _, _>(
            "SELECT COUNT(*) FROM comments WHERE date > ?",
            [start.format("%Y-%m-%d").to_string()],
            |row| row.get(0),
        )?;

        let o_new = conn.query_row::<i32, _, _>(
            "SELECT COUNT(*) FROM objects WHERE date > ?",
            [start.format("%Y-%m-%d").to_string()],
            |row| row.get(0),
        )?;

        Ok(format!(
            "评价总数：{}, 实体客体总数：{}, 月新增客体数：{}, 月增评价数：{}",
            c_count, o_count, o_new, m_new
        ))
    }
}

impl ObjComment {}

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
    println!("{:#?}", db.find_objteacher_with_id("918863e1af3b1e67"));
}

#[test]
fn my_test2() {
    assert_eq!("admin".to_owned(), SourceCate::Admin.to_string());
    assert_eq!("nest".to_owned(), CommentType::Nest.to_string());
}
