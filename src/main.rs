mod config;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use actix_web::http::header;
use actix_web::http::header::{DispositionParam, DispositionType, QualityItem};
use anyhow::Context;
use icalendar::Calendar;
use tracing::{info, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use crate::config::{AppConfig, Config};

const APP_NAME: &str = "tu-planner";

#[get("/")]
async fn calendar(config: web::Data<AppConfig>) -> impl Responder {
    let tiss_link = config.tiss.link();
    let response = reqwest::get(tiss_link).await.unwrap();
    let calendar = response.text().await.unwrap();
    let calendar: Calendar = calendar.parse().unwrap();
    HttpResponse::Ok()
        .content_type("text/calendar")
        .insert_header(header::ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::Filename("personal.ics".to_string())],
        })
        .insert_header(header::ContentLanguage(vec![QualityItem::max(config.tiss.locale().unwrap().into())]))
        .body(format!("{calendar}"))
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .context("Failed to set default tracing subscriber")?;

    let Config { app: app_config, service: service_config } = Config::load().context("Failed to load config")?;

    info!("actix-web {APP_NAME}: listening on {}", service_config.bind);

    HttpServer::new(move || {
        let test = App::new()
            .service(calendar)
            .app_data(web::Data::new(app_config.clone()));
        test
    })
        .bind(service_config.bind)?
        .run()
        .await?;
    Ok(())
}
