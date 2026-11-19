mod catalog;

use std::{error::Error, sync::Arc, time::SystemTime};

use askama::Template;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use catalog::{Calendar, Catalog, Day, Observance, Saint};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::{America, Pacific, Tz};
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

struct CalendarDay<'a> {
    date: NaiveDate,
    entries: Vec<CalendarEntry<'a>>,
}

struct CalendarMonth<'a> {
    start: NaiveDate,
    days: Vec<CalendarDay<'a>>,
}

#[derive(Template)]
#[template(path = "calendar.html")]
struct CalendarPage<'a> {
    calendar: &'a Calendar,
    months: Vec<CalendarMonth<'a>>,
}

#[derive(Template)]
#[template(path = "today.html")]
struct TodayPage<'a> {
    calendar: &'a Calendar,
    date: NaiveDate,
    selected_zone: &'static Zone,
    zones: &'static [Zone],
    entries: Vec<CalendarEntry<'a>>,
}

struct Zone {
    time_zone: Tz,
    label: &'static str,
}

const CENTRAL_TIME: Zone = Zone {
    time_zone: America::Chicago,
    label: "Central Time",
};

const ZONES: &[Zone] = &[
    Zone {
        time_zone: America::New_York,
        label: "Eastern Time",
    },
    CENTRAL_TIME,
    Zone {
        time_zone: America::Denver,
        label: "Mountain Time",
    },
    Zone {
        time_zone: America::Phoenix,
        label: "Arizona Time",
    },
    Zone {
        time_zone: America::Los_Angeles,
        label: "Pacific Time",
    },
    Zone {
        time_zone: America::Anchorage,
        label: "Alaska Time",
    },
    Zone {
        time_zone: Pacific::Honolulu,
        label: "Hawaii Time",
    },
];

#[derive(Deserialize)]
struct TodayParams {
    zone: Option<String>,
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
        .route("/today", get(today))
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
    let months = catalog
        .calendar
        .days
        .chunk_by(|left, right| left.date.month() == right.date.month())
        .map(|days| CalendarMonth {
            start: days[0].date,
            days: days
                .iter()
                .map(|day| CalendarDay {
                    date: day.date,
                    entries: calendar_entries(&catalog, day),
                })
                .collect(),
        })
        .collect();
    render(CalendarPage {
        calendar: &catalog.calendar,
        months,
    })
}

async fn today(State(catalog): State<AppState>, Query(params): Query<TodayParams>) -> Response {
    today_at(&catalog, params.zone.as_deref(), SystemTime::now().into())
}

fn today_at(catalog: &Catalog, zone_name: Option<&str>, now: DateTime<Utc>) -> Response {
    let selected_zone = ZONES
        .iter()
        .find(|zone| Some(zone.time_zone.name()) == zone_name)
        .unwrap_or(&CENTRAL_TIME);
    let date = now.with_timezone(&selected_zone.time_zone).date_naive();
    let entries = catalog
        .calendar
        .day(date)
        .map(|day| calendar_entries(catalog, day))
        .unwrap_or_default();
    let mut page = render(TodayPage {
        calendar: &catalog.calendar,
        date,
        selected_zone,
        zones: ZONES,
        entries,
    });
    page.headers_mut().insert(
        header::CACHE_CONTROL,
        axum::http::HeaderValue::from_static("no-store"),
    );
    page
}

fn calendar_entries<'a>(catalog: &'a Catalog, day: &'a Day) -> Vec<CalendarEntry<'a>> {
    day.observances
        .iter()
        .map(|observance| CalendarEntry {
            observance,
            saints: observance
                .saints
                .iter()
                .filter_map(|id| catalog.saint(id))
                .collect(),
        })
        .collect()
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
    fn directory_has_150_sourced_profiles() {
        let catalog = Catalog::load().unwrap();
        assert_eq!(catalog.saints.len(), 150);
        assert!(catalog.saints.iter().all(|saint| !saint.sources.is_empty()));
    }

    #[test]
    fn search_finds_name_variants_and_disambiguates_shared_names() {
        let catalog = Catalog::load().unwrap();
        assert_eq!(catalog.search("St. Therese")[0].id, "therese-of-lisieux");
        assert_eq!(catalog.search("Mother Teresa")[0].id, "teresa-of-calcutta");
        assert_eq!(catalog.search("Đũng Lạc")[0].id, "andrew-dung-lac");
        assert_eq!(
            catalog.search("Benoit Joseph Labre")[0].id,
            "benedict-joseph-labre"
        );
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
    fn annual_calendar_keeps_precedence_and_regional_alternatives() {
        let catalog = Catalog::load().unwrap();
        let calendar = &catalog.calendar;
        let observances = |date: &str| &calendar.day(date.parse().unwrap()).unwrap().observances;

        assert_eq!(calendar.days.len(), 365);
        assert!(observances("2026-04-05")[0].title.contains("Easter"));
        assert_eq!(observances("2026-11-01")[0].title, "All Saints");
        assert_eq!(observances("2026-11-22")[0].rank, "Solemnity");
        assert_eq!(
            observances("2026-11-23")
                .iter()
                .filter(|entry| entry.optional)
                .count(),
            3
        );
        assert!(observances("2026-11-29")[0].title.contains("Advent"));
        assert_eq!(observances("2026-11-30")[0].saints, ["andrew-the-apostle"]);
        assert!(
            observances("2026-09-05")
                .iter()
                .any(|entry| entry.optional && entry.title.contains("Teresa of Calcutta"))
        );
        assert!(
            observances("2026-10-09")
                .iter()
                .any(|entry| entry.optional && entry.title.contains("John Henry Newman"))
        );

        for date in ["2026-05-14", "2026-05-17"] {
            let primary: Vec<_> = observances(date)
                .iter()
                .filter(|entry| !entry.optional)
                .collect();
            assert_eq!(primary.len(), 2);
            assert!(
                primary
                    .iter()
                    .any(|entry| entry.title.contains("Ascension"))
            );
            assert!(primary.iter().all(|entry| entry.note.is_some()));
        }
    }

    #[tokio::test]
    async fn public_routes_render_results_and_cross_links() {
        let app = app(Arc::new(Catalog::load().unwrap()));
        let (status, home) = request(&app, "/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(home.contains("150 sourced profiles"));
        assert!(home.contains("action=\"/search\""));
        assert_eq!(home.matches("class=\"saint-card\"").count(), 150);
        assert!(home.contains("href=\"/today\""));

        let (status, results) = request(&app, "/search?q=St.+Therese").await;
        assert_eq!(status, StatusCode::OK);
        assert!(results.contains("/saints/therese-of-lisieux"));

        let (status, profile) = request(&app, "/saints/frances-xavier-cabrini").await;
        assert_eq!(status, StatusCode::OK);
        assert!(profile.contains("/calendar#day-2026-11-13"));
        assert!(profile.contains("Sources"));

        let (status, calendar) = request(&app, "/calendar").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(calendar.matches("class=\"calendar-day\"").count(), 365);
        assert!(calendar.contains("href=\"#month-12\""));
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

    #[tokio::test]
    async fn today_uses_the_selected_zones_civil_date_and_shows_coverage_gaps() {
        let catalog = Catalog::load().unwrap();
        let now = "2026-11-04T05:30:00Z".parse().unwrap();
        let central = today_at(&catalog, None, now);
        assert_eq!(central.headers()[header::CACHE_CONTROL], "no-store");
        let central = response_text(central).await;
        assert!(central.contains("datetime=\"2026-11-03\""));
        assert!(central.contains("Martin de Porres"));
        assert!(central.contains("Optional memorial"));

        let eastern = response_text(today_at(&catalog, Some("America/New_York"), now)).await;
        assert!(eastern.contains("datetime=\"2026-11-04\""));
        assert!(eastern.contains("Charles Borromeo"));
        assert!(eastern.contains("/calendar#day-2026-11-04"));

        let date_outside_coverage = "2027-01-02T12:00:00Z".parse().unwrap();
        let uncovered = response_text(today_at(&catalog, None, date_outside_coverage)).await;
        assert!(uncovered.contains("verified schedule covers January 1 through December 31, 2026"));
        assert!(!uncovered.contains("class=\"calendar-entry"));
    }

    async fn request(app: &Router, uri: &str) -> (StatusCode, String) {
        let request = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        let status = response.status();
        (status, response_text(response).await)
    }

    async fn response_text(response: Response) -> String {
        let body = to_bytes(response.into_body(), 1_000_000).await.unwrap();
        String::from_utf8(body.to_vec()).unwrap()
    }
}
