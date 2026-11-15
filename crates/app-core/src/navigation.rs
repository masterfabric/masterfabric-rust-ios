//! Navigasyon durumu. Hangi ekranın açık olduğuna Rust karar verir.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Route {
    Home,
    Detail { item_id: u32 },
    Benchmark,
}

impl Route {
    /// Platformun navigasyon yoluna (ör. SwiftUI `NavigationStack(path:)`) verilen sabit anahtar.
    pub fn key(&self) -> String {
        match self {
            Route::Home => "home".to_string(),
            Route::Detail { item_id } => format!("detail:{item_id}"),
            Route::Benchmark => "benchmark".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavEntry {
    pub key: String,
    pub route: Route,
}

/// Geri yığını. `stack[0]` her zaman `Home`'dur ve asla çıkarılmaz.
#[derive(Clone, Debug)]
pub struct Navigator {
    stack: Vec<Route>,
}

impl Default for Navigator {
    fn default() -> Self {
        Self {
            stack: vec![Route::Home],
        }
    }
}

impl Navigator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Yeni ekran aç. Aynı ekran zaten en üstteyse tekrar eklenmez (çift dokunma koruması).
    pub fn push(&mut self, route: Route) {
        if self.current() != &route {
            self.stack.push(route);
        }
    }

    /// Bir geri git. Kök ekranda hiçbir şey yapmaz. Geri gidildiyse `true`.
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    /// Kökten sonra yalnızca `path_len` ekran kalsın.
    pub fn pop_to(&mut self, path_len: usize) {
        self.stack.truncate(path_len + 1);
    }

    pub fn current(&self) -> &Route {
        self.stack
            .last()
            .expect("navigasyon yığını asla boş değildir")
    }

    pub fn stack(&self) -> &[Route] {
        &self.stack
    }

    pub fn entries(&self) -> Vec<NavEntry> {
        self.stack
            .iter()
            .map(|r| NavEntry {
                key: r.key(),
                route: r.clone(),
            })
            .collect()
    }
}
