mod config;

use std::sync::LazyLock;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_web::http::header;
use actix_web::http::header::{DispositionParam, DispositionType, QualityItem};
use anyhow::Context;
use icalendar::{Calendar, CalendarComponent, Component};
use regex::Regex;
use tracing::{info, Level};
use tracing_subscriber::{EnvFilter, FmtSubscriber};
use crate::config::{AppConfig, Config, Locale};

const APP_NAME: &str = "tu-planner";
const APP_ENV_NAME: LazyLock<String> = LazyLock::new(|| {
    APP_NAME.to_uppercase().chars().into_iter()
        .map(|c| {
            if c == '-' {
                '_'
            } else {
                c
            }
        })
        .collect()
});

const SPK_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new("\\WSPK\\W").unwrap());

fn calendar_response(calendar: Calendar, locale: Locale) -> HttpResponse {
    let filename = "personal.ics".to_string();
    HttpResponse::Ok()
        .content_type("text/calendar")
        .insert_header(header::ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![DispositionParam::Filename(filename)],
        })
        .insert_header(header::ContentLanguage(vec![QualityItem::max(locale.into())]))
        .body(calendar.to_string())
}

async fn calendar(config: web::Data<AppConfig>) -> impl Responder {
    let tiss_link = config.tiss.link();
    let response = reqwest::get(tiss_link).await.unwrap();
    let calendar = response.text().await.unwrap();
    let mut calendar: Calendar = calendar.parse().unwrap();

    calendar.components.retain(|component| {
        match component {
            CalendarComponent::Event(event) => {
                if let Some(description) = event.get_description() {
                    !SPK_REGEX.is_match(description)
                } else {
                    true
                }
            },
            _ => true,
        }
    });
    
    calendar_response(calendar, config.tiss.locale().unwrap())
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
            .route("/", web::get().to(calendar))
            .app_data(web::Data::new(app_config.clone()));
        test
    })
        .bind(service_config.bind)?
        .run()
        .await?;
    Ok(())
}
