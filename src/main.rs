mod story;

use crate::story::Story;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use actix_web_static_files::ResourceFiles;
use serde::{Deserialize, Serialize};

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

fn stories_dir() -> String {
    format!(
        "{}/stories",
        std::env::var("SPAM_DIR").expect("Must set SPAM_DIR environment variable")
    )
}

fn assets_dir() -> String {
    format!("{}/assets", stories_dir())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();

    let port = 8080;

    println!(
        "Serving from {} on port {port}",
        std::env::var("SPAM_DIR").expect("Must set SPAM_DIR environment variable")
    );

    dotenv::from_path(format!("{}/.env", std::env::var("SPAM_DIR").unwrap())).ok();

    HttpServer::new(move || {
        let generated = generate();
        App::new()
            .app_data(web::PayloadConfig::new(50 * 1024 * 1024))
            .service(
                web::scope("/api")
                    .service(list)
                    .service(read)
                    .service(write)
                    .service(rename)
                    .service(new)
                    .service(delete)
                    .service(key)
                    .service(prompt)
                    .service(upload)
                    .service(list_assets)
                    .service(get_asset)
                    .service(delete_asset),
            )
            .service(ResourceFiles::new("/", generated))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}

#[post("/list")]
async fn list() -> impl Responder {
    let paths = std::fs::read_dir(stories_dir()).expect("could not find directory ./stories");

    let mut dir_entries = paths
        .into_iter()
        .map(|f| f.unwrap())
        .filter(|e| {
            e.file_name()
                .into_string()
                .map(|n| n.ends_with(".json"))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    dir_entries.sort_by_key(|d| d.metadata().unwrap().modified().unwrap());
    dir_entries.reverse();

    let result = dir_entries
        .into_iter()
        .map(|e| e.file_name().into_string().unwrap().replace(".json", ""))
        .collect::<Vec<_>>();

    web::Json(result)
}

#[post("/read")]
async fn read(name: web::Json<String>) -> impl Responder {
    web::Json(read_story(&name.0))
}

#[post("/write")]
async fn write(body: web::Json<Story>) -> impl Responder {
    let story = body.0;
    write_story(&story.title.clone(), story);
    HttpResponse::Ok()
}

#[derive(Deserialize, Serialize, Default)]
struct RenamePayload {
    from: String,
    to: String,
}

#[post("/rename")]
async fn rename(body: web::Json<RenamePayload>) -> impl Responder {
    rename_story(&body.from, &body.to);

    HttpResponse::Ok().await
}

#[post("/new")]
async fn new(body: web::Json<String>) -> impl Responder {
    let name = body.0;
    write_story(&name.clone(), Story::new(name));
    HttpResponse::Ok().await
}

#[post("/delete")]
async fn delete(body: web::Json<String>) -> impl Responder {
    let name = body.0;
    delete_story(&name);
    HttpResponse::Ok().await
}

#[post("/key")]
async fn key() -> impl Responder {
    web::Json(std::env::var("OPENROUTER_API_KEY").unwrap())
}

#[post("/prompt")]
async fn prompt() -> impl Responder {
    std::fs::read_to_string(format!("{}/prompt.txt", std::env::var("SPAM_DIR").unwrap())).unwrap()
}

fn is_safe_component(s: &str) -> bool {
    !s.is_empty() && !s.contains('/') && !s.contains('\\') && s != "." && s != ".."
}

#[post("/upload/{story}/{filename}")]
async fn upload(path: web::Path<(String, String)>, body: web::Bytes) -> impl Responder {
    let (story, filename) = path.into_inner();
    if !is_safe_component(&story) || !is_safe_component(&filename) {
        return HttpResponse::BadRequest().body("invalid path component");
    }
    let dir = format!("{}/{}", assets_dir(), story);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return HttpResponse::InternalServerError().body(format!("mkdir failed: {e}"));
    }
    let file_path = format!("{}/{}", dir, filename);
    if let Err(e) = std::fs::write(&file_path, &body) {
        return HttpResponse::InternalServerError().body(format!("write failed: {e}"));
    }
    HttpResponse::Ok().finish()
}

#[post("/assets/list")]
async fn list_assets(body: web::Json<String>) -> impl Responder {
    let story = body.0;
    if !is_safe_component(&story) {
        let empty: Vec<String> = vec![];
        return HttpResponse::BadRequest().json(empty);
    }
    let dir = format!("{}/{}", assets_dir(), story);
    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => {
            let empty: Vec<String> = vec![];
            return HttpResponse::Ok().json(empty);
        }
    };
    let mut result: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().ok().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    result.sort();
    HttpResponse::Ok().json(result)
}

fn image_mime(filename: &str) -> &'static str {
    let lower = filename.to_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".avif") {
        "image/avif"
    } else if lower.ends_with(".bmp") {
        "image/bmp"
    } else {
        "application/octet-stream"
    }
}

#[get("/assets/{story}/{filename}")]
async fn get_asset(path: web::Path<(String, String)>) -> impl Responder {
    let (story, filename) = path.into_inner();
    if !is_safe_component(&story) || !is_safe_component(&filename) {
        return HttpResponse::BadRequest().finish();
    }
    let file_path = format!("{}/{}/{}", assets_dir(), story, filename);
    match std::fs::read(&file_path) {
        Ok(bytes) => HttpResponse::Ok()
            .content_type(image_mime(&filename))
            .body(bytes),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[post("/assets/delete/{story}/{filename}")]
async fn delete_asset(path: web::Path<(String, String)>) -> impl Responder {
    let (story, filename) = path.into_inner();
    if !is_safe_component(&story) || !is_safe_component(&filename) {
        return HttpResponse::BadRequest().finish();
    }
    let file_path = format!("{}/{}/{}", assets_dir(), story, filename);
    match std::fs::remove_file(&file_path) {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => HttpResponse::NotFound().finish(),
        Err(e) => HttpResponse::InternalServerError().body(format!("delete failed: {e}")),
    }
}

fn read_story(name: &str) -> Story {
    let path = format!("{}/{name}.json", stories_dir());
    serde_json::from_str::<Story>(
        &std::fs::read_to_string(path.clone())
            .unwrap_or_else(|_| panic!("could not find file: {path}")),
    )
    .unwrap()
}

fn write_story(name: &str, story: Story) {
    let path = format!("{}/{name}.json", stories_dir());
    let str = serde_json::to_string(&story).unwrap();
    std::fs::write(path, str).unwrap();
}

fn rename_story(from: &str, to: &str) {
    let mut story = read_story(from);
    story.title = to.to_string();
    write_story(to, story);

    let path = format!("{}/{from}.json", stories_dir());
    std::fs::remove_file(path).unwrap();

    let from_assets = format!("{}/{from}", assets_dir());
    let to_assets = format!("{}/{to}", assets_dir());
    if std::path::Path::new(&from_assets).exists() {
        std::fs::rename(&from_assets, &to_assets).unwrap();
    }
}

fn delete_story(name: &str) {
    let path = format!("{}/{name}.json", stories_dir());
    std::fs::remove_file(path).unwrap();

    let story_assets = format!("{}/{name}", assets_dir());
    if std::path::Path::new(&story_assets).exists() {
        std::fs::remove_dir_all(&story_assets).unwrap();
    }
}
