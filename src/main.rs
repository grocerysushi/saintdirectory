mod catalog;

use std::{error::Error, sync::Arc};

use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use catalog::{Calendar, Catalog, Day, Observance, Saint};
use serde::Deserialize;

type AppState = Arc<Catalog>;

#[derive(Template)]
#[template(path = "directory.html")]
struct DirectoryPage<'a> {
    saints: Vec<&'a Saint>,
    count: usize,
    query: &'a str,
    is_search: bool,
    calendar_title: &'a str,
}

#[derive(Template)]
#[template(path = "profile.html")]
struct ProfilePage<'a> {
    saint: &'a Saint,
    calendar: &'a Calendar,
    observances: Vec<(&'a Day, &'a Observance)>,
}

struct CalendarEntry<'a> {
    observance: &'a Observance,
    saints: Vec<&'a Saint>,
}

#[derive(Template)]
#[template(path = "calendar.html")]
struct CalendarPage<'a> {
    calendar: &'a Calendar,
    schedule: Vec<(&'a Day, Vec<CalendarEntry<'a>>)>,
}

#[derive(Template)]
#[template(path = "not_found.html")]
struct NotFoundPage;

#[derive(Deserialize)]
struct SearchParams {
    #[serde(default)]
    q: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let catalog = Arc::new(Catalog::load()?);
    let bind = std::env::var("SAINT_DIRECTORY_ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(bind).await?;
    println!(
        "Saint Directory is listening at http://{}",
        listener.local_addr()?
    );
    axum::serve(listener, app(catalog)).await?;
    Ok(())
}

fn app(catalog: AppState) -> Router {
    Router::new()
        .route("/", get(directory))
        .route("/search", get(search))
        .route("/saints/{id}", get(profile))
        .route("/calendar", get(calendar))
        .route("/assets/site.css", get(styles))
        .route("/assets/paradise.jpg", get(artwork))
        .fallback(not_found)
        .with_state(catalog)
}

async fn directory(State(catalog): State<AppState>) -> Response {
    render(DirectoryPage {
        saints: catalog.saints.iter().collect(),
        count: catalog.saints.len(),
        query: "",
        is_search: false,
        calendar_title: &catalog.calendar.title,
    })
}

async fn search(State(catalog): State<AppState>, Query(params): Query<SearchParams>) -> Response {
    let query: String = params.q.trim().chars().take(120).collect();
    let saints = if query.is_empty() {
        catalog.saints.iter().collect()
    } else {
        catalog.search(&query)
    };
    render(DirectoryPage {
        count: saints.len(),
        saints,
        query: &query,
        is_search: true,
        calendar_title: &catalog.calendar.title,
    })
}

async fn profile(State(catalog): State<AppState>, Path(id): Path<String>) -> Response {
    let Some(saint) = catalog.saint(&id) else {
        return (StatusCode::NOT_FOUND, render(NotFoundPage)).into_response();
    };
    let observances = catalog
        .calendar
        .days
        .iter()
        .flat_map(|day| day.observances.iter().map(move |entry| (day, entry)))
        .filter(|(_, entry)| entry.saints.contains(&id))
        .collect();
    render(ProfilePage {
        saint,
        calendar: &catalog.calendar,
        observances,
    })
}

async fn calendar(State(catalog): State<AppState>) -> Response {
    let schedule = catalog
        .calendar
        .days
        .iter()
        .map(|day| {
            let entries = day
                .observances
                .iter()
                .map(|observance| CalendarEntry {
                    observance,
                    saints: observance
                        .saints
                        .iter()
                        .filter_map(|id| catalog.saint(id))
                        .collect(),
                })
                .collect();
            (day, entries)
        })
        .collect();
    render(CalendarPage {
        calendar: &catalog.calendar,
        schedule,
    })
}

async fn styles() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../static/site.css"),
    )
}

async fn artwork() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "image/jpeg")],
        include_bytes!("../static/paradise.jpg").as_slice(),
    )
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, render(NotFoundPage))
}

fn render(template: impl Template) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(error) => {
            eprintln!("Template rendering failed: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, "Something went wrong.").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[test]
    fn first_release_has_25_sourced_profiles() {
        let catalog = Catalog::load().unwrap();
        assert_eq!(catalog.saints.len(), 25);
        assert!(catalog.saints.iter().all(|saint| !saint.sources.is_empty()));
    }

    #[test]
    fn search_finds_name_variants_and_disambiguates_shared_names() {
        let catalog = Catalog::load().unwrap();
        assert_eq!(catalog.search("St. Therese")[0].id, "therese-of-lisieux");
        assert_eq!(catalog.search("Mother Teresa")[0].id, "teresa-of-calcutta");
        assert_eq!(catalog.search("Đũng Lạc")[0].id, "andrew-dung-lac");
        let results = catalog.search("saint anthony");
        let ids: Vec<_> = results
            .iter()
            .take(2)
            .map(|saint| saint.id.as_str())
            .collect();
        assert!(ids.contains(&"anthony-of-padua"));
        assert!(ids.contains(&"anthony-the-great"));
    }

    #[test]
    fn november_calendar_preserves_sunday_precedence_and_optional_memorials() {
        let catalog = Catalog::load().unwrap();
        let days = &catalog.calendar.days;
        assert_eq!(days.len(), 30);
        assert_eq!(days[0].observances[0].title, "All Saints");
        assert_eq!(days[21].observances[0].rank, "Solemnity");
        assert!(
            days[21].observances[0]
                .title
                .contains("King of the Universe")
        );
        assert_eq!(
            days[22]
                .observances
                .iter()
                .filter(|entry| entry.optional)
                .count(),
            3
        );
        assert!(days[28].observances[0].title.contains("Advent"));
        assert_eq!(days[29].observances[0].saints, ["andrew-the-apostle"]);
    }

    #[tokio::test]
    async fn public_routes_render_results_and_cross_links() {
        let app = app(Arc::new(Catalog::load().unwrap()));
        let (status, home) = request(&app, "/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(home.contains("25 sourced profiles"));
        assert!(home.contains("action=\"/search\""));
        assert_eq!(home.matches("class=\"saint-card\"").count(), 25);

        let (status, results) = request(&app, "/search?q=St.+Therese").await;
        assert_eq!(status, StatusCode::OK);
        assert!(results.contains("/saints/therese-of-lisieux"));

        let (status, profile) = request(&app, "/saints/frances-xavier-cabrini").await;
        assert_eq!(status, StatusCode::OK);
        assert!(profile.contains("/calendar#day-2026-11-13"));
        assert!(profile.contains("Sources"));

        let (status, calendar) = request(&app, "/calendar").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(calendar.matches("class=\"calendar-day\"").count(), 30);
        assert!(calendar.contains("/saints/frances-xavier-cabrini"));
        assert!(calendar.contains("2026cal.pdf"));

        let (status, empty) = request(&app, "/search?q=%3Cscript%3Ezzzxqz%3C%2Fscript%3E").await;
        assert_eq!(status, StatusCode::OK);
        assert!(empty.contains("0 profiles found"));
        assert!(!empty.contains("<script>"));
        assert_eq!(
            request(&app, "/saints/unknown").await.0,
            StatusCode::NOT_FOUND
        );
    }

    async fn request(app: &Router, uri: &str) -> (StatusCode, String) {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let body = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }
}
