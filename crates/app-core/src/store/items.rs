//! Data layer ("store"): the single source of truth for app data.

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: u32,
    pub title: String,
    pub subtitle: String,
    pub body: String,
    pub category: String,
    /// SF Symbol name (iOS); Android maps it to a matching icon.
    pub symbol: String,
    pub tags: Vec<String>,
    pub favorite: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ItemStore {
    items: Vec<Item>,
}

impl ItemStore {
    pub fn with_items(items: Vec<Item>) -> Self {
        Self { items }
    }

    /// Sample data: projects you can build with Rust.
    pub fn seeded() -> Self {
        // (title, subtitle, body, category, SF Symbol, tags)
        type Seed = (
            &'static str,
            &'static str,
            &'static str,
            &'static str,
            &'static str,
            &'static [&'static str],
        );
        let raw: &[Seed] = &[
            (
                "Shared Rust Core",
                "One core, native screens",
                "State, navigation and business rules live in Rust; iOS and Android only render. This app itself is built this way.",
                "Mobile",
                "square.stack.3d.up.fill",
                &["uniffi", "swiftui", "compose"],
            ),
            (
                "ESP32-C6 Sensor Node",
                "no_std with Embassy + esp-hal",
                "A low-power embedded device that measures temperature and humidity and sends it to a phone over BLE.",
                "Embedded",
                "sensor.fill",
                &["embassy", "ble", "no_std"],
            ),
            (
                "Raspberry Pi 5 Robot Brain",
                "Camera, path finding, Wi-Fi control",
                "Rust services on Linux; delegates motor control to a Pico 2 over UART.",
                "Robotics",
                "cpu.fill",
                &["rppal", "tokio", "uart"],
            ),
            (
                "Pico 2 Motor Controller",
                "Real-time loop on the RP2350",
                "PWM motor driving, encoder reading and PID control. Async with embassy-rp.",
                "Embedded",
                "gearshape.2.fill",
                &["embassy-rp", "pwm", "pid"],
            ),
            (
                "Fast CLI Tools",
                "Single binary, no dependencies",
                "Command-line tools that scan, filter and report on large log and CSV files.",
                "Tools",
                "terminal.fill",
                &["clap", "rayon"],
            ),
            (
                "Local AI Server Dashboard",
                "Monitor models on your Macs in one screen",
                "A small service that shows status, GPU/RAM usage and queues of AI servers on the network.",
                "Tools",
                "chart.xyaxis.line",
                &["axum", "tokio"],
            ),
            (
                "Encrypted Sync",
                "End-to-end encrypted core",
                "An end-to-end encrypted backup and sync core that only transfers changed chunks.",
                "Mobile",
                "lock.shield.fill",
                &["crypto", "sqlite"],
            ),
            (
                "2D Game with Bevy",
                "ECS architecture on Metal",
                "Physics and simulation experiments with Rust's Bevy game engine.",
                "Games",
                "gamecontroller.fill",
                &["bevy", "ecs"],
            ),
        ];
        let items = raw
            .iter()
            .enumerate()
            .map(
                |(i, (title, subtitle, body, category, symbol, tags))| Item {
                    id: i as u32 + 1,
                    title: title.to_string(),
                    subtitle: subtitle.to_string(),
                    body: body.to_string(),
                    category: category.to_string(),
                    symbol: symbol.to_string(),
                    tags: tags.iter().map(|t| t.to_string()).collect(),
                    favorite: false,
                },
            )
            .collect();
        Self { items }
    }

    pub fn all(&self) -> &[Item] {
        &self.items
    }

    pub fn get(&self, id: u32) -> Option<&Item> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Toggles favorite. `None` if the item does not exist, otherwise the new state.
    pub fn toggle_favorite(&mut self, id: u32) -> Option<bool> {
        let item = self.items.iter_mut().find(|i| i.id == id)?;
        item.favorite = !item.favorite;
        Some(item.favorite)
    }

    /// Business rule: search is case-insensitive over title, subtitle, category and tags;
    /// favorites first, then by id.
    pub fn visible(&self, query: &str) -> Vec<&Item> {
        let q = query.trim().to_lowercase();
        let mut out: Vec<&Item> = self
            .items
            .iter()
            .filter(|i| {
                q.is_empty()
                    || i.title.to_lowercase().contains(&q)
                    || i.subtitle.to_lowercase().contains(&q)
                    || i.category.to_lowercase().contains(&q)
                    || i.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect();
        out.sort_by_key(|i| (!i.favorite, i.id));
        out
    }

    pub fn related(&self, id: u32, limit: usize) -> Vec<&Item> {
        let Some(item) = self.get(id) else {
            return vec![];
        };
        self.items
            .iter()
            .filter(|i| i.id != id && i.category == item.category)
            .take(limit)
            .collect()
    }
}
