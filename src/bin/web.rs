//! 目前为 demo 阶段，只是为了验证可行性
//!

use actix_cors::Cors;
use actix_web::http::header;
use actix_web::{get, Responder};
use actix_web::{web, App, HttpResponse, HttpServer};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

use safc::db::*;

const PORT: u16 = 11096;

#[derive(Debug, Serialize, Deserialize)]
struct ApiQuery {
    school_cate: Option<String>,
    university: Option<String>,
    department: Option<String>,
    supervisor: Option<String>,
}

#[get("/api")]
async fn hello(db: web::Data<SAFCdb>) -> impl Responder {
    let result = db.db_status().unwrap();
    HttpResponse::Ok().json(result)
}

#[get("/api/download/db")]
async fn download_file(db: web::Data<SAFCdb>) -> impl Responder {
    let db_path = PathBuf::from(db.get_db_path());

    // 安全检查：确保文件存在且可读
    match File::open(&db_path) {
        Ok(mut file) => {
            let mut contents = Vec::new();
            match file.read_to_end(&mut contents) {
                Ok(_) => {
                    // 从路径中提取文件名
                    let filename = db_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("database.db");

                    // 返回数据库文件
                    HttpResponse::Ok()
                        .append_header(header::ContentType::octet_stream())
                        .append_header(header::ContentDisposition::attachment(filename))
                        .body(contents)
                }
                Err(_) => HttpResponse::InternalServerError().json("无法读取数据库文件"),
            }
        }
        Err(_) => HttpResponse::NotFound().json("数据库文件不存在"),
    }
}

#[get("/api/query")]
async fn api_query(db: web::Data<SAFCdb>, item: web::Query<ApiQuery>) -> impl Responder {
    let q = item.into_inner();
    if q.school_cate.is_none() {
        return HttpResponse::Ok().json(db.find_school_cate().unwrap());
    }
    if q.university.is_none() {
        return HttpResponse::Ok().json(db.find_university(&q.school_cate.unwrap()).unwrap());
    }
    if q.department.is_none() {
        return HttpResponse::Ok().json(
            db.find_department(&q.school_cate.unwrap(), &q.university.unwrap())
                .unwrap(),
        );
    }
    if q.supervisor.is_none() {
        return HttpResponse::Ok().json(
            db.find_supervisor(
                &q.school_cate.unwrap(),
                &q.university.unwrap(),
                &q.department.unwrap(),
            )
            .unwrap(),
        );
    }
    let obj_teacher = db
        .find_object_with_path(
            &q.university.unwrap(),
            &q.department.unwrap(),
            &q.supervisor.unwrap(),
        )
        .unwrap()
        .unwrap(); // TODO thread 'actix-rt|system:0|arbiter:1' panicked at 'called `Option::unwrap()` on a `None` value', src/bin/web.rs:61:10

    HttpResponse::Ok().json(db.find_comment(&obj_teacher.object_id).unwrap())
    // todo 这里需初步的格式化一下以显示嵌套评价
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    dotenv().ok();
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    log::info!("Starting SAFT web server at PORT{} ... by Framecraft", PORT);

    // connect to SQLite DB
    let db = SAFCdb::new();

    // start HTTP server
    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin() // 允许任何源
            .allow_any_method() // 允许任何 HTTP 方法
            .allow_any_header() // 允许任何头部
            .supports_credentials() // 支持凭证
            .max_age(3600); // 预检请求的缓存时间

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(db.clone()))
            // .wrap(middleware::Logger::default())
            .service(hello)
            .service(api_query)
            .service(download_file) // 添加数据库下载服务
    })
    .bind(("127.0.0.1", PORT))?
    .run()
    .await
}
