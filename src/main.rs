use serenity::Client;
use serenity::all::GatewayIntents;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

mod voice_tracker;
mod calc;
use crate::voice_tracker::{ChannelActivityTracker, VoiceHandler};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();
    
    let token = std::env::var("DISCORD_TOKEN")
        .expect("DISCORD_TOKEN이 .env 파일에 설정되어야 합니다");

    let intents = GatewayIntents::GUILDS 
        | GatewayIntents::GUILD_VOICE_STATES;

    let mut client = Client::builder(&token, intents)
        .event_handler(VoiceHandler)
        .type_map_insert::<ChannelActivityTracker>(Arc::new(RwLock::new(HashMap::new())))
        .await
        .expect("클라이언트 생성 실패");

    println!("봇을 시작합니다...");

    if let Err(why) = client.start().await {
        println!("클라이언트 에러: {:?}", why);
    }
}