//! Kullanıcı olayları. Platform katmanı yalnızca bunları gönderir.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Ana sayfadaki bir öğeye dokunuldu.
    OpenDetail { item_id: u32 },
    /// Benchmark ekranını aç.
    OpenBenchmark,
    /// Bir ekran geri git.
    Back,
    /// Platformun navigasyon yığını kısaldığında (ör. kaydırarak geri) senkronize et.
    /// `path_len`: kökten (Home) sonraki ekran sayısı.
    PopTo { path_len: u32 },
    /// Ana sayfa arama metni.
    SetQuery { query: String },
    /// Favori aç/kapat (iş kuralı: favoriler listede üstte).
    ToggleFavorite { item_id: u32 },
    /// Tek bir benchmark'ı çalıştır.
    RunBenchmark { id: String },
    /// Tüm benchmark'ları sırayla çalıştır.
    RunAllBenchmarks,
    /// Benchmark sonuçlarını temizle.
    ResetBenchmarks,
}
