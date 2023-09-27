//! 目前为 demo 阶段，只是为了验证可行性
//!

use actix_cors::Cors;
use actix_web::{get, Responder};
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::io;

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
        .unwrap();

    return HttpResponse::Ok().json(db.find_comment(&obj_teacher.object_id).unwrap());
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
        let cors = Cors::permissive(); // todo
        App::new()
            .wrap(cors)
            .app_data(web::Data::new(db.clone()))
            .wrap(middleware::Logger::default())
            .service(hello)
            .service(api_query)
    })
    .bind(("127.0.0.1", PORT))?
    .run()
    .await
}
