use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::model::id::{ChannelId, GuildId, UserId};
use serenity::model::voice::VoiceState;
use serenity::prelude::*;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use tokio::sync::Mutex;

struct Handler {
    is_in_voice_channel: Arc<Mutex<HashMap<UserId, bool>>>,
    notified_channel_ids: Arc<Mutex<HashMap<GuildId, ChannelId>>>,
}

#[async_trait]
impl EventHandler for Handler {
    async fn voice_state_update(&self, ctx: Context, _: Option<VoiceState>, new: VoiceState) {
        // 入退室以外にミュートやスピーカーミュートでもイベントが発火することに注意

        if let Some(_) = new.channel_id {
            // 入室
            if let Some(member) = new.member {
                // ボイスチャンネルに参加しているかを確認
                let mut lock = self.is_in_voice_channel.lock().await;
                // 参加していた場合、処理を終了
                if lock.get(&member.user.id) == Some(&true) {
                    return;
                }

                lock.insert(member.user.id, true);

                // 入室通知
                if let Some(guild_id) = new.guild_id {
                    // 通知するテキストチャンネルの取得
                    let lock = self.notified_channel_ids.lock().await;
                    match lock.get(&guild_id) {
                        Some(channel_id) => {
                            let msg = format!("{} が入室しました", member.user.name);
                            if let Err(err) = channel_id.say(&ctx.http, msg).await {
                                println!("error sending message: {:?}", err);
                            }
                        }
                        None => (),
                    }
                }
            }
        } else {
            // 退出
            if let Some(member) = new.member {
                let mut lock = self.is_in_voice_channel.lock().await;
                lock.insert(member.user.id, false);

                // 退室通知
                if let Some(guild_id) = new.guild_id {
                    // 通知するテキストチャンネルの取得
                    let lock = self.notified_channel_ids.lock().await;
                    match lock.get(&guild_id) {
                        Some(channel_id) => {
                            let msg = format!("{} が退室しました", member.user.name);
                            if let Err(err) = channel_id.say(&ctx.http, msg).await {
                                println!("error sending message: {:?}", err);
                            }
                        }
                        None => (),
                    }
                }
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        // 通知先のテキストチャンネルの登録
        // "tutti"という名前のテキストチャンネルを登録する
        for guild in ready.guilds.iter() {
            match guild.id.channels(&ctx.http).await {
                Ok(channels) => {
                    for (channel_id, guild_channel) in channels.into_iter() {
                        if guild_channel.name() == "tutti" {
                            let mut lock = self.notified_channel_ids.lock().await;
                            lock.insert(guild.id, channel_id);
                        }
                    }
                }
                Err(err) => println!(
                    "error getting notified channel id in {}: {:?}",
                    guild.id, err
                ),
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("expected a token in the environment");

    let intents = GatewayIntents::GUILD_VOICE_STATES | GatewayIntents::GUILD_MESSAGES;
    let mut client = Client::builder(&token, intents)
        .event_handler(Handler {
            is_in_voice_channel: Arc::new(Mutex::new(HashMap::new())),
            notified_channel_ids: Arc::new(Mutex::new(HashMap::new())),
        })
        .await
        .expect("error creating client");

    if let Err(err) = client.start().await {
        println!("client error: {:?}", err);
    }
}
