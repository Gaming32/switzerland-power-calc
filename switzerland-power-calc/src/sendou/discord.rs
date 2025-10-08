use crate::sendou::lang::Language;
use dashmap::DashMap;
use serenity::all::{
    Cache, CacheHttp, CommandId, Context, CreateInteractionResponse,
    CreateInteractionResponseMessage, EventHandler, GuildId, Http, Interaction, UserId,
};
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::oneshot;

pub struct DiscordEventHandler {
    pub ready: Mutex<Option<oneshot::Sender<()>>>,
    pub language_command: Arc<RwLock<Option<CommandId>>>,
    pub language_output: Arc<DashMap<UserId, Language>>,
}

#[serenity::async_trait]
impl EventHandler for DiscordEventHandler {
    async fn cache_ready(&self, _: Context, _: Vec<GuildId>) {
        if let Some(sender) = self.ready.lock().unwrap().take() {
            let _ = sender.send(());
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        let Some(command) = interaction.command() else {
            return;
        };
        if Some(command.data.id) != *self.language_command.read().unwrap() {
            return;
        }
        let language = command.data.options.first().map_or_else(
            || Language::from_discord_id(&command.locale),
            |lang| lang.value.as_str().and_then(Language::from_id),
        );
        let response = if let Some(language) = language {
            self.language_output.insert(command.user.id, language);
            language.changed_language(language)
        } else {
            "Unfortunately your language is unsupported. Please run the command again and select one of the listed options.".into()
        };
        let response = CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new().content(response),
        );
        if let Err(e) = command.create_response(ctx, response).await {
            println!("Failed to send user language change response: {e}");
        };
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
