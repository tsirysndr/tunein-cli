use actix_cors::Cors;
use actix_web::{
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    web, App, HttpRequest, HttpResponse, HttpServer,
};
use anyhow::Error;
use async_graphql::http::GraphiQLSource;
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use owo_colors::OwoColorize;
use rust_embed::RustEmbed;

use self::graphql::{build_schema, AppSchema};

pub mod graphql;

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Assets;

async fn graphql_handler(schema: web::Data<AppSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphiql() -> HttpResponse {
    HttpResponse::Ok()
        .insert_header((CONTENT_TYPE, "text/html; charset=utf-8"))
        .body(GraphiQLSource::build().endpoint("/graphql").finish())
}

fn serve_embedded(path: &str) -> HttpResponse {
    // Fall back to index.html for SPA routes (e.g. /browse/music).
    let asset = Assets::get(path).or_else(|| Assets::get("index.html"));
    match asset {
        Some(content) => {
            let mime = mime_guess::from_path(if Assets::get(path).is_some() {
                path
            } else {
                "index.html"
            })
            .first_or_octet_stream();
            let cache = if path.starts_with("assets/") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            HttpResponse::Ok()
                .insert_header((CONTENT_TYPE, mime.as_ref()))
                .insert_header((CACHE_CONTROL, cache))
                .body(content.data.into_owned())
        }
        None => HttpResponse::NotFound().body(
            "Web UI not built. Run `bun run build` in the `web` directory and rebuild tunein-cli.",
        ),
    }
}

async fn spa(req: HttpRequest) -> HttpResponse {
    let path = req.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    serve_embedded(path)
}

pub async fn exec(port: u16) -> Result<(), Error> {
    let schema = build_schema();
    println!(
        "{}",
        r#"
        ______              ____       _______   ____
       /_  __/_ _____  ___ /  _/__    / ___/ /  /  _/
        / / / // / _ \/ -_)/ // _ \  / /__/ /___/ /
       /_/  \_,_/_//_/\__/___/_//_/  \___/____/___/

    "#
        .bright_green()
    );
    println!(
        "Web UI available at {}",
        format!("http://localhost:{}", port).cyan()
    );
    println!(
        "GraphQL playground available at {}",
        format!("http://localhost:{}/graphql", port).cyan()
    );

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(schema.clone()))
            .wrap(Cors::permissive())
            .route("/graphql", web::post().to(graphql_handler))
            .route("/graphql", web::get().to(graphiql))
            .default_service(web::get().to(spa))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await?;

    Ok(())
}
