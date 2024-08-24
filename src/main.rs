mod story;

use actix_web::{web, App, HttpServer, Responder, post, HttpResponse};
use actix_web_static_files::ResourceFiles;
use serde::{Deserialize, Serialize};
use crate::story::Story;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok().unwrap();
    HttpServer::new(move || {
        let generated = generate();
        App::new()
            .service(
                web::scope("/api")
                    .service(list)
                    .service(read)
                    .service(write)
                    .service(rename)
                    .service(new)
                    .service(delete)
                    .service(key)
            )
            .service(ResourceFiles::new("/", generated))
    })
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

#[post("/list")]
async fn list() -> impl Responder {
    let paths = std::fs::read_dir("./stories").unwrap();

    let mut dir_entries = paths.into_iter()
        .map(|f| f.unwrap())
        .collect::<Vec<_>>();

    dir_entries.sort_by_key(|d| d.metadata().unwrap().modified().unwrap());
    dir_entries.reverse();

    let result = dir_entries.into_iter()
        .map(|e| e
            .file_name()
            .into_string()
            .unwrap()
            .replace(".json", ""))
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
    to: String
}

#[post("/rename")]
async fn rename(body: web::Json<RenamePayload>) -> impl Responder {
    rename_story(&*body.from, &*body.to);

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

fn read_story(name: &str) -> Story {
    let path = format!("stories/{name}.json");
    serde_json::from_str::<Story>(&*std::fs::read_to_string(path.clone()).expect(&format!("could not find file: {path}"))).unwrap()
}

fn write_story(name: &str, story: Story) {
    let path = format!("stories/{name}.json");
    let str = serde_json::to_string(&story).unwrap();
    std::fs::write(path, str).unwrap();
}

fn rename_story(from: &str, to: &str) {
    let mut story = read_story(from);
    story.title = to.to_string();
    write_story(to, story);

    let path = format!("stories/{from}.json");
    std::fs::remove_file(path).unwrap();
}

fn delete_story(name: &str) {
    let path = format!("stories/{name}.json");
    std::fs::remove_file(path).unwrap();
}
