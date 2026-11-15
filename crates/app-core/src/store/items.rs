//! Veri katmanı ("store"). Discord'daki store yapısı gibi tek doğruluk kaynağı.

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub id: u32,
    pub title: String,
    pub subtitle: String,
    pub body: String,
    pub category: String,
    /// SF Symbol adı (iOS) — Android tarafında eşleşen ikon seçilir.
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

    /// Örnek veri: Rust ile yapılabilecek projeler.
    pub fn seeded() -> Self {
        // (başlık, alt başlık, açıklama, kategori, SF Symbol, etiketler)
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
                "Paylaşılan Rust Çekirdeği",
                "Discord modeli: tek çekirdek, yerel ekranlar",
                "Durum, navigasyon ve iş kuralları Rust'ta; iOS ve Android yalnızca çizer. Bu uygulamanın kendisi bu modelle yazıldı.",
                "Mobil",
                "square.stack.3d.up.fill",
                &["uniffi", "swiftui", "compose"],
            ),
            (
                "ESP32-C6 Sensör Düğümü",
                "Embassy + esp-hal ile no_std",
                "Sıcaklık ve nemi ölçüp BLE ile telefona gönderen düşük güçlü gömülü cihaz.",
                "Gömülü",
                "sensor.fill",
                &["embassy", "ble", "no_std"],
            ),
            (
                "Raspberry Pi 5 Robot Beyni",
                "Kamera, yol bulma, Wi‑Fi kontrol",
                "Linux üzerinde Rust servisleri; motorları UART ile Pico 2'ye devreder.",
                "Robotik",
                "cpu.fill",
                &["rppal", "tokio", "uart"],
            ),
            (
                "Pico 2 Motor Kontrolcüsü",
                "RP2350 üzerinde gerçek zamanlı döngü",
                "PWM ile motor sürme, enkoder okuma ve PID kontrol. embassy-rp ile async.",
                "Gömülü",
                "gearshape.2.fill",
                &["embassy-rp", "pwm", "pid"],
            ),
            (
                "Hızlı CLI Araçları",
                "Tek dosya, bağımlılıksız çıktı",
                "Büyük log ve CSV dosyalarını tarayan, filtreleyen ve raporlayan komut satırı araçları.",
                "Araçlar",
                "terminal.fill",
                &["clap", "rayon"],
            ),
            (
                "Yerel AI Sunucu Paneli",
                "Mac'lerdeki modelleri tek ekranda izle",
                "Ağdaki AI sunucularının durumunu, GPU/RAM kullanımını ve kuyruğunu gösteren küçük bir servis.",
                "Araçlar",
                "chart.xyaxis.line",
                &["axum", "tokio"],
            ),
            (
                "Şifreli Senkronizasyon",
                "libsignal / 1Password yaklaşımı",
                "Uçtan uca şifreli, yalnızca değişen parçaları aktaran yedekleme ve senkronizasyon çekirdeği.",
                "Mobil",
                "lock.shield.fill",
                &["crypto", "sqlite"],
            ),
            (
                "Bevy ile 2D Oyun",
                "ECS mimarisi, Metal üzerinde",
                "Rust'ın oyun motoru Bevy ile fizik ve simülasyon denemeleri.",
                "Oyun",
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

    /// Favori durumunu değiştirir. Öğe yoksa `None`, varsa yeni durum.
    pub fn toggle_favorite(&mut self, id: u32) -> Option<bool> {
        let item = self.items.iter_mut().find(|i| i.id == id)?;
        item.favorite = !item.favorite;
        Some(item.favorite)
    }

    /// İş kuralı: arama başlık, alt başlık, kategori ve etiketlerde (büyük/küçük harf
    /// duyarsız) yapılır; favoriler önce, sonra id sırası.
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
