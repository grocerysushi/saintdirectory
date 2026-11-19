# Catholic Saint Directory

A server-rendered Rust directory with 279 sourced profiles, name and keyword search, and the 2026 and 2027 U.S. Roman Rite calendars. The calendars link to individual saints, named archangels, and commemorated groups of martyrs and founders. Search matches alternate names, accent-insensitive spellings, places, vocations, patronage, and profile text.

## Run locally

Run the app with a stable Rust toolchain.

```sh
cargo run --locked
```

Open [http://127.0.0.1:3000](http://127.0.0.1:3000). Set `SAINT_DIRECTORY_ADDR` to change the listen address. Rust serves HTML and CSS, with navigation and search available without JavaScript. The binary embeds its dataset and assets.

Open `/today` for the current day's observances. It defaults to U.S. Central Time and offers seven U.S. time zones. The selected zone determines the date and year, including at midnight on New Year's Day. Verified coverage runs from January 1, 2026, through December 31, 2027.

Browse either year at `/calendar/2026` and `/calendar/2027`. The original `/calendar` URL still opens 2026 and supports existing date links.

## Edit and check the content

- `data/saints*.toml` files hold permanent profile IDs, original biographies, aliases, and source links.
- Each `data/calendar-*.toml` file covers a complete civil year. The records distinguish primary celebrations from optional observances, include regional alternatives for Ascension, and link to available profiles. [USCCB 2026 calendar](https://www.usccb.org/resources/2026cal.pdf) · [USCCB 2027 calendar](https://www.usccb.org/resources/2027cal.pdf)

Keep existing IDs when changing titles, add sources for new profiles, and rebuild the binary after editing the data. Startup validates profile records, date coverage, and cross-references.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The homepage artwork is Giovanni di Paolo’s *Paradise* (1445), Rogers Fund, 1906, object 06.1046. The Metropolitan Museum of Art provides the public-domain image through its Open Access program. [Collection record](https://www.metmuseum.org/art/collection/search/436512)
