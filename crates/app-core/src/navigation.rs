//! Navigation state. Rust decides which screen is open.

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Route {
    Home,
    Detail { item_id: u32 },
    Benchmark,
    Info,
}

impl Route {
    /// Stable key used in the platform navigation path (e.g. SwiftUI `NavigationStack(path:)`).
    pub fn key(&self) -> String {
        match self {
            Route::Home => "home".to_string(),
            Route::Detail { item_id } => format!("detail:{item_id}"),
            Route::Benchmark => "benchmark".to_string(),
            Route::Info => "info".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavEntry {
    pub key: String,
    pub route: Route,
}

/// Back stack. `stack[0]` is always `Home` and is never removed.
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

    /// Open a screen. Not pushed again if already on top (double-tap guard).
    pub fn push(&mut self, route: Route) {
        if self.current() != &route {
            self.stack.push(route);
        }
    }

    /// Go back one screen. No-op at the root. Returns `true` if it popped.
    pub fn pop(&mut self) -> bool {
        if self.stack.len() > 1 {
            self.stack.pop();
            true
        } else {
            false
        }
    }

    /// Keep only `path_len` screens after the root.
    pub fn pop_to(&mut self, path_len: usize) {
        self.stack.truncate(path_len + 1);
    }

    pub fn current(&self) -> &Route {
        self.stack.last().expect("navigation stack is never empty")
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
