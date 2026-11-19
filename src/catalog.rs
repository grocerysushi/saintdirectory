use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::io;

use chrono::{Datelike, NaiveDate};
use serde::Deserialize;
use unicode_normalization::{UnicodeNormalization, char::is_combining_mark};

const PROFILE_FILES: &[&str] = &[
    include_str!("../data/saints.toml"),
    include_str!("../data/saints-founders.toml"),
    include_str!("../data/saints-witnesses.toml"),
    include_str!("../data/saints-apostles-and-confessors.toml"),
    include_str!("../data/saints-calendar-winter-spring.toml"),
    include_str!("../data/saints-calendar-summer.toml"),
    include_str!("../data/saints-calendar-autumn-advent.toml"),
    include_str!("../data/saints-calendar-groups.toml"),
    include_str!("../data/saints-eastern-christianity.toml"),
    include_str!("../data/saints-modern-witnesses.toml"),
    include_str!("../data/saints-africa-asia-pacific.toml"),
    include_str!("../data/saints-americas.toml"),
];
const CALENDAR_FILES: &[&str] = &[
    include_str!("../data/calendar-2026.toml"),
    include_str!("../data/calendar-2027.toml"),
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Saint {
    pub id: String,
    pub name: String,
    pub aliases: Vec<String>,
    pub years: String,
    pub vocation: String,
    pub place: String,
    pub summary: String,
    pub biography: Vec<String>,
    #[serde(default)]
    pub patronage: Vec<String>,
    pub sources: Vec<Source>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    pub title: String,
    pub url: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Profiles {
    saints: Vec<Saint>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Calendar {
    pub year: i32,
    pub title: String,
    pub name: String,
    pub source_title: String,
    pub source_url: String,
    pub days: Vec<Day>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Day {
    pub date: NaiveDate,
    pub observances: Vec<Observance>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Observance {
    pub title: String,
    pub rank: String,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub saints: Vec<String>,
    pub note: Option<String>,
}

impl Calendar {
    pub fn day(&self, date: NaiveDate) -> Option<&Day> {
        let index = self.days.binary_search_by_key(&date, |day| day.date).ok()?;
        self.days.get(index)
    }
}

pub struct Catalog {
    pub saints: Vec<Saint>,
    pub calendars: BTreeMap<i32, Calendar>,
}

impl Catalog {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        let mut saints = Vec::new();
        for file in PROFILE_FILES {
            let profiles: Profiles = toml::from_str(file)?;
            saints.extend(profiles.saints);
        }
        let ids = validate_profiles(&saints)?;
        let mut calendars = BTreeMap::new();
        for file in CALENDAR_FILES {
            let calendar: Calendar = toml::from_str(file)?;
            validate_calendar(&calendar, &ids)?;
            let year = calendar.year;
            if calendars.insert(year, calendar).is_some() {
                return Err(invalid(format!("Duplicate calendar year: {year}")).into());
            }
        }
        saints.sort_by_cached_key(|saint| normalize(&saint.name));
        Ok(Self { saints, calendars })
    }

    pub fn saint(&self, id: &str) -> Option<&Saint> {
        self.saints.iter().find(|saint| saint.id == id)
    }

    pub fn search(&self, query: &str) -> Vec<&Saint> {
        let query = normalize(query);
        if query.is_empty() {
            return Vec::new();
        }

        let terms: Vec<_> = query.split_whitespace().collect();
        let mut matches: Vec<_> = self
            .saints
            .iter()
            .filter_map(|saint| rank(saint, &query, &terms).map(|rank| (rank, saint)))
            .collect();
        matches.sort_by_key(|(rank, _)| *rank);
        matches.into_iter().map(|(_, saint)| saint).collect()
    }
}

fn normalize(text: &str) -> String {
    let letters: String = text
        .nfkd()
        .filter(|ch| !is_combining_mark(*ch))
        .flat_map(char::to_lowercase)
        .map(|ch| match ch {
            'đ' => 'd',
            ch if ch.is_alphanumeric() => ch,
            _ => ' ',
        })
        .collect();
    letters
        .split_whitespace()
        .filter(|word| !matches!(*word, "st" | "saint"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn rank(saint: &Saint, query: &str, terms: &[&str]) -> Option<u8> {
    let name = normalize(&saint.name);
    let aliases: Vec<_> = saint.aliases.iter().map(|alias| normalize(alias)).collect();

    if name == query {
        return Some(0);
    }
    if aliases.iter().any(|alias| alias == query) {
        return Some(1);
    }
    if std::iter::once(&name)
        .chain(&aliases)
        .any(|name| terms_match(terms, name))
    {
        return Some(2);
    }

    let topics = normalize(&format!(
        "{} {} {}",
        saint.vocation,
        saint.place,
        saint.patronage.join(" "),
    ));
    if terms_match(terms, &topics) {
        return Some(3);
    }

    let text = normalize(&format!(
        "{} {} {} {} {}",
        name,
        aliases.join(" "),
        topics,
        saint.summary,
        saint.biography.join(" "),
    ));
    terms_match(terms, &text).then_some(4)
}

fn terms_match(terms: &[&str], text: &str) -> bool {
    terms
        .iter()
        .all(|term| text.split_whitespace().any(|word| word.starts_with(term)))
}

fn validate_profiles(saints: &[Saint]) -> io::Result<HashSet<&str>> {
    let mut ids = HashSet::new();
    for saint in saints {
        let valid_id = !saint.id.is_empty()
            && saint.id.split('-').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
            });
        if !valid_id || !ids.insert(saint.id.as_str()) {
            return Err(invalid(format!(
                "Invalid or duplicate saint ID: {}",
                saint.id
            )));
        }

        let fields = [
            &saint.name,
            &saint.years,
            &saint.vocation,
            &saint.place,
            &saint.summary,
        ];
        if fields.iter().any(|field| field.trim().is_empty())
            || saint.biography.is_empty()
            || saint
                .biography
                .iter()
                .any(|paragraph| paragraph.trim().is_empty())
            || saint.sources.is_empty()
            || saint
                .sources
                .iter()
                .any(|source| source.title.trim().is_empty() || !is_source_url(&source.url))
        {
            return Err(invalid(format!("Incomplete profile: {}", saint.id)));
        }
    }
    Ok(ids)
}

fn validate_calendar(calendar: &Calendar, ids: &HashSet<&str>) -> io::Result<()> {
    let first = NaiveDate::from_ymd_opt(calendar.year, 1, 1)
        .ok_or_else(|| invalid(format!("Invalid calendar year: {}", calendar.year)))?;
    let last = NaiveDate::from_ymd_opt(calendar.year, 12, 31)
        .ok_or_else(|| invalid(format!("Invalid calendar year: {}", calendar.year)))?;
    if calendar.days.len() != last.ordinal() as usize || !is_source_url(&calendar.source_url) {
        return Err(invalid(format!(
            "The {} calendar needs a source and one entry for each date",
            calendar.year
        )));
    }
    for (day, date) in calendar.days.iter().zip(first.iter_days()) {
        if day.date != date || day.observances.is_empty() {
            return Err(invalid(format!(
                "Missing or out-of-order calendar entry for {date}"
            )));
        }
        if day.observances.iter().all(|entry| entry.optional) {
            return Err(invalid(format!("Missing primary observance for {date}")));
        }
        for entry in &day.observances {
            if entry.title.trim().is_empty() || entry.rank.trim().is_empty() {
                return Err(invalid(format!("Incomplete observance for {date}")));
            }
            for id in &entry.saints {
                if !ids.contains(id.as_str()) {
                    return Err(invalid(format!("Unknown saint ID {id} on {date}")));
                }
            }
        }
    }
    Ok(())
}

fn invalid(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn is_source_url(url: &str) -> bool {
    url.strip_prefix("https://")
        .is_some_and(|address| !address.is_empty() && !address.chars().any(char::is_whitespace))
}
