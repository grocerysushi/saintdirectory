# Catholic Saint Directory

A server-rendered Rust directory with 25 cited saint profiles, name and keyword search, and the November 2026 U.S. Roman Rite calendar. Search matches alternate names, accent-insensitive spellings, places, vocations, patronage, and biographical text.

## Run locally

Run the app with a stable Rust toolchain.

```sh
cargo run --locked
```

Open [http://127.0.0.1:3000](http://127.0.0.1:3000). Set `SAINT_DIRECTORY_ADDR` to change the listen address. Rust serves HTML and CSS, with navigation and search available without JavaScript. The binary embeds its dataset and assets.

## Edit and check the content

- `data/saints.toml` holds permanent profile IDs, original biographies, aliases, and source links.
- `data/calendar.toml` holds the complete civil month, with primary and optional observances marked separately and links to the profiles where present. It follows the November section of the [USCCB 2026 liturgical calendar](https://www.usccb.org/resources/2026cal.pdf).

Keep existing IDs when changing titles, add sources for new profiles, and rebuild the binary after editing the data. Startup validates profile records, date coverage, and cross-references.

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

The homepage artwork is Giovanni di Paolo’s *Paradise* (1445), Rogers Fund, 1906, object 06.1046. The Metropolitan Museum of Art provides the public-domain image through its Open Access program. [Collection record](https://www.metmuseum.org/art/collection/search/436512)
