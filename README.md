# Catholic Saint Directory

A server-rendered Rust directory with 150 cited saint profiles, name and keyword search, and the full 2026 U.S. Roman Rite calendar. Search matches alternate names, accent-insensitive spellings, places, vocations, patronage, and biographical text.

## Run locally

Run the app with a stable Rust toolchain.

```sh
cargo run --locked
```

Open [http://127.0.0.1:3000](http://127.0.0.1:3000). Set `SAINT_DIRECTORY_ADDR` to change the listen address. Rust serves HTML and CSS, with navigation and search available without JavaScript. The binary embeds its dataset and assets.

Open `/today` for the current day's observances. It defaults to U.S. Central Time and offers seven U.S. time zones. The local date follows the selected zone's daylight-saving rules. Verified coverage runs from January 1 through December 31, 2026. The page shows the coverage dates when the current date falls outside that year.

## Edit and check the content

- `data/saints*.toml` files hold permanent profile IDs, original biographies, aliases, and source links.
- `data/calendar-2026.toml` holds all 365 dates of the civil year. It distinguishes primary celebrations from optional observances, records the regional alternatives for Ascension, and links to available profiles. [USCCB 2026 liturgical calendar, February 2026 emendation](https://www.usccb.org/resources/2026cal.pdf)

Keep existing IDs when changing titles, add sources for new profiles, and rebuild the binary after editing the data. Startup validates profile records, date coverage, and cross-references.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The homepage artwork is Giovanni di Paolo’s *Paradise* (1445), Rogers Fund, 1906, object 06.1046. The Metropolitan Museum of Art provides the public-domain image through its Open Access program. [Collection record](https://www.metmuseum.org/art/collection/search/436512)
