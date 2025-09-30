use serenity::async_trait;
use serenity::all::CommandInteraction;
use serenity::all::Command;
use serenity::all::CommandOptionType;
use serenity::all::CreateCommand;
use serenity::all::CreateCommandOption;
use serenity::all::CreateInteractionResponse;
use serenity::all::CreateInteractionResponseMessage;
use serenity::all::CommandDataOptionValue;
use serenity::all::Interaction;
use serenity::all::Ready;
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

// 보이스 채널의 활성화 시작 시간을 추적
pub struct ChannelActivityTracker {
    pub(crate) active_channels: Arc<RwLock<HashMap<u64, Instant>>>,
}

impl ChannelActivityTracker {
    pub fn new() -> Self {
        Self {
            active_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl TypeMapKey for ChannelActivityTracker {
    type Value = Arc<RwLock<HashMap<u64, Instant>>>;
}

pub fn new_tracker_store() -> Arc<RwLock<HashMap<u64, Instant>>> {
    Arc::new(RwLock::new(HashMap::new()))
}

pub struct VoiceHandler;

#[async_trait]
impl EventHandler for VoiceHandler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{}님의 봇이 준비되었습니다!", ready.user.name);
        // 슬래시 커맨드 등록: /calc
        let cmd = CreateCommand::new("calc")
            .description("수식을 PEMDAS 우선순위로 계산합니다")
            .add_option(
                CreateCommandOption::new(CommandOptionType::String, "expr", "계산할 수식")
                    .required(true),
            );

        if let Err(e) = Command::create_global_command(&ctx.http, cmd)
        .await
        {
            eprintln!("/calc 등록 실패: {:?}", e);
        }

        // 길드 커맨드로도 즉시 등록 (봇이 속한 모든 길드)
        for guild_id in ctx.cache.guilds() {
            let guild_cmd = CreateCommand::new("calc")
                .description("입력된 수식을 계산합니다.")
                .add_option(
                    CreateCommandOption::new(CommandOptionType::String, "expr", "계산할 수식")
                        .required(true),
                );
            if let Err(e) = guild_id.create_command(&ctx.http, guild_cmd).await {
                eprintln!("/calc 길드 등록 실패 ({}): {:?}", guild_id, e);
            }
        }
    }

    async fn voice_state_update(
        &self,
        ctx: Context,
        old: Option<VoiceState>,
        new: VoiceState,
    ) {
        let data = ctx.data.read().await;
        let tracker = data
            .get::<ChannelActivityTracker>()
            .expect("활동 추적기를 찾을 수 없습니다")
            .clone();
        drop(data);

        let guild_id = match new.guild_id {
            Some(id) => id,
            None => return,
        };

        let user = match new.member {
            Some(ref member) => &member.user,
            None => return,
        };

        // 텍스트 채널 ID (알림을 보낼 채널)
        // 여기를 실제 텍스트 채널 ID로 변경하세요
        let notification_channel_id = serenity::model::id::ChannelId::new(1422179903373185094);
        
        // 멘션할 역할 ID (선택사항)
        let mention_role_id = serenity::model::id::RoleId::new(1422182421415202879);

        match (old.as_ref().and_then(|v| v.channel_id), new.channel_id) {
            // 보이스 채널에 입장
            (None, Some(channel_id)) | (Some(_), Some(channel_id)) 
                if old.as_ref().and_then(|v| v.channel_id) != Some(channel_id) => {
                
                let channel_name = get_channel_name(&ctx, guild_id, channel_id).await;
                
                // 채널의 현재 인원 수 확인
                let member_count = count_voice_members(&ctx, guild_id, channel_id).await;
                
                let mut tracker_lock = tracker.write().await;
                
                // 첫 번째 사람이 입장한 경우
                if member_count == 1 {
                    tracker_lock.insert(channel_id.get(), Instant::now());
                    
                    let _ = notification_channel_id
                        .say(
                            &ctx.http,
                            format!(
                                "🟢 **#{}** 방이 활성화되었습니다. <@&{}>",
                                channel_name, mention_role_id
                            ),
                        )
                        .await;
                }
                
                // 입장 알림
                let _ = notification_channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "➡️ {} 님이 **#{}** 에 입장했습니다.",
                            user.name, channel_name
                        ),
                    )
                    .await;
            }

            // 보이스 채널에서 퇴장
            (Some(old_channel_id), None) => {
                let channel_name = get_channel_name(&ctx, guild_id, old_channel_id).await;
                
                // 퇴장 알림
                let _ = notification_channel_id
                    .say(
                        &ctx.http,
                        format!(
                            "⬅️ {} 님이 **#{}** 방에서 퇴장했습니다.",
                            user.name, channel_name
                        ),
                    )
                    .await;
                
                // 채널의 현재 인원 수 확인
                let member_count = count_voice_members(&ctx, guild_id, old_channel_id).await;
                
                // 마지막 사람이 퇴장한 경우
                if member_count == 0 {
                    let mut tracker_lock = tracker.write().await;
                    
                    if let Some(start_time) = tracker_lock.remove(&old_channel_id.get()) {
                        let duration = start_time.elapsed();
                        let hours = duration.as_secs() / 3600;
                        let minutes = (duration.as_secs() % 3600) / 60;
                        let seconds = duration.as_secs() % 60;
                        
                        let _ = notification_channel_id
                            .say(
                                &ctx.http,
                                format!(
                                    "🔴 **#{}** 방이 비활성화되었습니다. 활성화 시간: {}시간 {}분 {}초",
                                    channel_name, hours, minutes, seconds
                                ),
                            )
                            .await;
                    }
                }
            }

            _ => {}
        }
    }

    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(cmd) = interaction {
            if cmd.data.name == "calc" {
                handle_calc(&ctx, &cmd).await;
            }
        }
    }
}

async fn handle_calc(ctx: &Context, cmd: &CommandInteraction) {
    // expr 옵션 추출
    let expr_val = cmd
        .data
        .options
        .iter()
        .find(|o| o.name == "expr")
        .and_then(|o| match &o.value {
            CommandDataOptionValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("");

    if expr_val.is_empty() {
        let _ = cmd
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("표현식을 입력하세요."),
                ),
            )
            .await;
        return;
    }

    let result_text = match crate::calc::evaluate(expr_val) {
        Ok(v) => format!("{} = {}", expr_val, v),
        Err(e) => format!("{} -> 오류: {}", expr_val, e),
    };

    let _ = cmd
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(result_text),
            ),
        )
        .await;
}

// 채널 이름 가져오기
async fn get_channel_name(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    channel_id: serenity::model::id::ChannelId,
) -> String {
    if let Some(guild) = ctx.cache.guild(guild_id) {
        if let Some(channel) = guild.channels.get(&channel_id) {
            return channel.name.clone();
        }
    }
    "알 수 없는 채널".to_string()
}

// 보이스 채널의 현재 인원 수 세기
async fn count_voice_members(
    ctx: &Context,
    guild_id: serenity::model::id::GuildId,
    channel_id: serenity::model::id::ChannelId,
) -> usize {
    if let Some(guild) = ctx.cache.guild(guild_id) {
        return guild
            .voice_states
            .values()
            .filter(|vs| vs.channel_id == Some(channel_id))
            .count();
    }
    0
}


