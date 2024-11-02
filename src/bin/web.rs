//! 目前为 demo 阶段，只是为了验证可行性
//!
use actix_cors::Cors;
use actix_web::body::BoxBody;
use actix_web::http::{header, StatusCode};
use actix_web::rt;
use actix_web::{get, post, Responder};
use actix_web::{web, App, HttpResponse, HttpServer};

use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use actix_governor::{Governor, GovernorConfigBuilder};
use actix_web::{
    dev::{ServiceRequest, ServiceResponse},
    middleware::{from_fn, Next},
    Error,
};
use safc::db::*;
use safc::sec;

const PORT: u16 = 11096;
const MAX_POST_PER_DAY: u64 = 20; // 每 IP 每天最多 20 次 POST 请求

lazy_static! {
    static ref BLOCK_DB: Mutex<HashMap<String, u64>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Serialize, Deserialize)]
struct ApiQuery {
    school_cate: Option<String>,
    university: Option<String>,
    department: Option<String>,
    supervisor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CreateCommentReq {
    school_cate: String,
    university: String,
    department: String,
    supervisor: String,
    content: String,
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
    let obj_teacher = match db
        .find_object_with_path(
            &q.university.unwrap(),
            &q.department.unwrap(),
            &q.supervisor.unwrap(),
        )
        .unwrap()
    {
        Some(t) => t,
        None => {
            return HttpResponse::NotFound().json("教师信息未找到");
        }
    };
    HttpResponse::Ok().json(db.find_comment(&obj_teacher.object_id).unwrap())
    // todo 这里需初步的格式化一下以显示嵌套评价
}

#[post("/api/new/comment")]
async fn new_comment(db: web::Data<SAFCdb>, form: web::Json<CreateCommentReq>) -> HttpResponse {
    let exist_teacher =
        match db.find_object_with_path(&form.university, &form.department, &form.supervisor) {
            Err(e) => {
                return HttpResponse::InternalServerError().json(e.to_string());
            }
            Ok(o) => match o {
                Some(t) => t,
                None => {
                    // 需要创建实体
                    let date = get_current_date();
                    let school_cate = form.school_cate.clone();
                    let university = form.university.clone();
                    let department = form.department.clone();
                    let supervisor = form.supervisor.clone();
                    let object_id = sec::hash_object_id(&university, &department, &supervisor);

                    let teacher = ObjTeacher {
                        school_cate,
                        university,
                        department,
                        supervisor,
                        date,
                        info: None,
                        object_id,
                    };
                    if let Err(e) = db.add_object(&teacher) {
                        return HttpResponse::InternalServerError().json(e.to_string());
                    };
                    teacher
                }
            },
        };

    let obj_comment = ObjComment::new_with_otp(
        exist_teacher.object_id,
        form.content.clone(),
        SourceCate::Web,
        CommentType::Teacher,
        "".to_string(), // TODO: 需要 OTP
    );

    match db.add_comment(&obj_comment) {
        Ok(_) => HttpResponse::Ok().json("评论成功"),
        Err(e) => HttpResponse::InternalServerError().json(e.to_string()),
    }
}

async fn block_middleware(
    req: ServiceRequest,
    next: Next<BoxBody>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    let headers = req.headers();
    let method = req.method();

    if method == actix_web::http::Method::POST {
        let origin_addr = headers.get("origin").and_then(|h| h.to_str().ok());
        let mut block_guard = BLOCK_DB.lock().unwrap();

        match origin_addr {
            Some(addr) => {
                let count = block_guard.entry(addr.to_string()).or_insert(0);
                *count += 1;
                if *count > MAX_POST_PER_DAY {
                    log::info!("限流：count:{}, origin:{}", *count, addr);
                    let response = HttpResponse::build(StatusCode::TOO_MANY_REQUESTS)
                        .body("超过每日 Post 请求次数限制");
                    return Ok(req.into_response(response.map_into_boxed_body()));
                }
            }
            None => {
                let response = HttpResponse::BadRequest().body("\"origin\"字段必须存在");
                return Ok(req.into_response(response.map_into_boxed_body()));
            }
        }
    }

    next.call(req).await
}

// 定期清理哈希表的函数
async fn clean_block_db() {
    log::info!("clean_block_db 启动");
    loop {
        // 每天 0 点清理一次
        tokio::time::sleep(Duration::from_secs(24 * 60 * 60)).await;
        if let Ok(mut guard) = BLOCK_DB.lock() {
            guard.clear();
            log::info!("已清理访问计数器");
        }
    }
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    // 初始化日志库
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    
    log::info!("Starting SAFT web server at PORT{} ... by Framecraft", PORT);

    // 启动清理任务
    rt::spawn(clean_block_db());

    // connect to SQLite DB
    let db = SAFCdb::new();

    // start HTTP server
    HttpServer::new(move || {
        // 限流配置：每个 IP 每分钟最多 60 次请求
        let governor_conf = GovernorConfigBuilder::default()
            .per_second(20) // 每秒请求数
            .burst_size(60) // 突发请求上限
            .finish()
            .unwrap();

        let cors = Cors::default()
            .allow_any_origin() // 允许任何源
            .allow_any_method() // 允许任何 HTTP 方法
            .allow_any_header() // 允许任何头部
            .supports_credentials() // 支持凭证
            .max_age(3600); // 预检请求的缓存时间

        App::new()
            .wrap(from_fn(block_middleware))
            .wrap(cors)
            .wrap(Governor::new(&governor_conf)) // 添加限流中间件
            .app_data(web::Data::new(db.clone()))
            .service(hello)
            .service(api_query)
            .service(download_file)
            .service(new_comment)
    })
    .bind(("127.0.0.1", PORT))?
    .run()
    .await
}
