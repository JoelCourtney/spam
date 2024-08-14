mod story;

use actix_web::{web, App, HttpServer, Responder, post, HttpResponse};
use actix_web_static_files::ResourceFiles;
use crate::story::Story;

include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        let generated = generate();
        App::new()
            .service(
                web::scope("/api")
                    .service(list)
                    .service(read)
                    .service(write)
                    .service(new)
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

    let result = paths.into_iter()
        .map(|f| f
            .unwrap()
            .file_name()
            .into_string()
            .unwrap()
            .replace(".json", ""))
        .collect::<Vec<_>>();

    web::Json(result)
}

#[post("/read")]
async fn read(name: web::Json<String>) -> impl Responder {
    web::Json(read_story(name.0))
}

#[post("/write")]
async fn write(body: web::Json<Story>) -> impl Responder {
    let story = body.0;
    write_story(story.title.clone(), story);
    HttpResponse::Ok()
}

#[post("/new")]
async fn new(body: web::Json<String>) -> impl Responder {
    let name = body.0;
    write_story(name, Story::default());
    HttpResponse::Ok().await
}

fn read_story(name: String) -> Story {
    let path = format!("stories/{name}.json");
    serde_json::from_str::<Story>(&*std::fs::read_to_string(path.clone()).expect(&format!("could not find file: {path}"))).unwrap()
}

fn write_story(name: String, story: Story) {
    let path = format!("stories/{name}.json");
    let str = serde_json::to_string(&story).unwrap();
    std::fs::write(path, str).unwrap();
}
