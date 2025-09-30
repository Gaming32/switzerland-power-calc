use serenity::all::{Cache, CacheHttp, Context, EventHandler, GuildId, Http};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub struct DiscordEventHandler {
    pub ready: Mutex<Option<oneshot::Sender<()>>>,
}

#[serenity::async_trait]
impl EventHandler for DiscordEventHandler {
    async fn cache_ready(&self, _: Context, _: Vec<GuildId>) {
        if let Some(sender) = self.ready.lock().unwrap().take() {
            let _ = sender.send(());
        }
    }
}

#[derive(Clone)]
pub struct DiscordHttp(Arc<Cache>, Arc<Http>);

impl DiscordHttp {
    pub fn new(cache: Arc<Cache>, http: Arc<Http>) -> Self {
        Self(cache, http)
    }

    pub fn cache(&self) -> &Cache {
        &self.0
    }
}

impl CacheHttp for DiscordHttp {
    fn http(&self) -> &Http {
        &self.1
    }

    fn cache(&self) -> Option<&Arc<Cache>> {
        Some(&self.0)
    }
}
