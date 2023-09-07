use crate::sec::*;
use rusqlite::{params, Connection, Result};

const DB_PATH: &str = "./db.sqlite"; // TODO

/// 来源分类：admin, urfire, telegram...
use strum_macros::{Display, EnumString};
#[derive(Debug, EnumString, Display)] // ?
pub enum SourceCate {
    #[strum(serialize = "admin")]
    Admin,
    #[strum(serialize = "urfire")]
    Urfire,
    #[strum(serialize = "telegram")]
    Telegram,
    #[strum(serialize = "web")]
    Web,
}

fn _db_open() -> Result<Connection> {
    Connection::open(DB_PATH)
}

pub fn find_school_cate() -> Result<Vec<String>> {
    let conn = _db_open()?;

    let mut stmt = conn.prepare("SELECT DISTINCT school_cate FROM objects")?;
    let rows = stmt.query_map([], |row| row.get::<usize, String>(0))?;

    rows.map(|x| x).collect::<Result<Vec<_>, _>>()
}

pub fn find_university(s_c: &String) -> Result<Vec<String>> {
    let conn = _db_open()?;

    let mut stmt =
        conn.prepare("SELECT DISTINCT university FROM objects WHERE school_cate=(?1)")?;
    let rows = stmt.query_map([s_c], |row| row.get(0))?;

    rows.map(|x| x).collect::<Result<Vec<_>, _>>()
}

pub fn find_department(s_c: &String, university: &String) -> Result<Vec<String>> {
    let conn = _db_open()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT department FROM objects WHERE \
        school_cate=(?1) AND university=(?2)",
    )?;
    let rows = stmt.query_map([s_c, university], |row| row.get::<_, String>(0))?;
    rows.map(|x| x).collect::<Result<Vec<_>, _>>()
}

pub fn find_supervisor(
    s_c: &String,
    university: &String,
    department: &String,
) -> Result<Vec<String>> {
    let conn = _db_open()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT supervisor FROM objects WHERE \
        school_cate=(?1) AND university=(?2)  AND department=(?3)",
    )?;
    let rows = stmt.query_map([s_c, university, department], |row| row.get::<_, String>(0))?;
    rows.map(|x| x).collect::<Result<Vec<_>, _>>()
}

pub fn find_object(
    university: &String,
    department: &String,
    supervisor: &String,
) -> Result<Vec<String>> {
    let conn = _db_open()?;

    let mut stmt = conn.prepare(
        "SELECT DISTINCT object FROM objects WHERE \
        supervisor=(?1) AND university=(?2) AND department=(?3)",
    )?;

    let rows = stmt.query_map([supervisor, university, department], |row| {
        row.get::<_, String>(0)
    })?;
    rows.map(|x| x).collect::<Result<Vec<_>, _>>()
}

pub fn get_comment(object_id: &String) -> Result<Vec<String>> {
    let conn = _db_open()?;

    let mut stmt =
        conn.prepare("SELECT description, date, source_cate, id FROM comments WHERE object=? ")?;
    let rows = stmt.query_map([object_id], |row| {
        Ok(format!(
            "Date: {} | From: {} | ID: {}\n评价：{}",
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(0)?
        ))
    })?;
    rows.map(|x| x).collect::<Result<Vec<_>, _>>()
}

use chrono;
/// 增加评价客体，有一些值在函数内计算
pub fn add_object_to_database(
    // conn: &Connection,
    school_cate: &String,
    university: &String,
    department: &String,
    supervisor: &String,
    data: &String,
) -> Result<(), rusqlite::Error> {
    let conn = _db_open()?;
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

pub fn add_comment_to_database(
    // conn: &Connection,
    object_id: &String,
    comment: &String,
    date: &String,
    source_cate: SourceCate,
    comment_type: &String,
    otp: &String,
) -> Result<(), rusqlite::Error> {
    let conn = _db_open()?;
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
    println!("{:#?}", get_comment(&"b148b44fd82fda41".to_string()));
    // add_object_to_database(
    // &"schoolcate".to_string(),
    // &"university".to_string(),
    // &"department".to_string(),
    // &"supervisor".to_string()
    // ).unwrap();
}
