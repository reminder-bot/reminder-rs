use chrono_tz::{Tz, TZ_VARIANTS};
use sea_orm_migration::prelude::*;

use crate::extension::postgres::Type;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000001_create_table"
    }
}

#[derive(Iden)]
pub enum Guild {
    Table,
    Id,
}

#[derive(Iden)]
pub enum Channel {
    Table,
    Id,
    GuildId,
    Nudge,
    WebhookId,
    WebhookToken,
    Paused,
    PausedUntil,
}

#[derive(Iden)]
pub enum User {
    Table,
    Id,
    DmChannel,
    Timezone,
}

#[derive(Iden)]
pub enum Reminder {
    Table,
    Id,
    Uid,
    Name,
    ChannelId,
    UtcTime,
    Timezone,
    IntervalSeconds,
    IntervalMonths,
    Enabled,
    Expires,
    Username,
    Avatar,
    Content,
    Tts,
    Attachment,
    AttachmentName,
    EmbedTitle,
    EmbedDescription,
    EmbedImageUrl,
    EmbedThumbnailUrl,
    EmbedFooter,
    EmbedFooterUrl,
    EmbedAuthor,
    EmbedAuthorUrl,
    EmbedColor,
    EmbedFields,
    SetAt,
    SetBy,
}

#[derive(Iden)]
pub enum ReminderTemplate {
    Table,
    Id,
    GuildId,
    Name,
    Username,
    Avatar,
    Content,
    Tts,
    Attachment,
    AttachmentName,
    EmbedTitle,
    EmbedDescription,
    EmbedImageUrl,
    EmbedThumbnailUrl,
    EmbedFooter,
    EmbedFooterUrl,
    EmbedAuthor,
    EmbedAuthorUrl,
    EmbedColor,
    EmbedFields,
}

#[derive(Iden)]
pub enum Timer {
    Table,
    Id,
    StartTime,
    Name,
    UserId,
    GuildId,
}

#[derive(Iden)]
pub enum Todo {
    Table,
    Id,
    UserId,
    GuildId,
    ChannelId,
    Value,
}

#[derive(Iden)]
pub enum CommandMacro {
    Table,
    Id,
    GuildId,
    Name,
    Description,
    Commands,
}

pub enum Timezone {
    Type,
    Tz(Tz),
}

impl Iden for Timezone {
    fn unquoted(&self, s: &mut dyn Write) {
        write!(
            s,
            "{}",
            match self {
                Self::Type => "timezone".to_string(),
                Self::Tz(tz) => tz.to_string(),
            }
        )
        .unwrap();
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_type(
                Type::create()
                    .as_enum(Timezone::Type)
                    .values(TZ_VARIANTS.iter().map(|tz| Timezone::Tz(tz.to_owned())))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Guild::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Guild::Id).big_integer().not_null().primary_key())
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Channel::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Channel::Id).big_integer().not_null().primary_key())
                    .col(ColumnDef::new(Channel::GuildId).big_integer())
                    .col(ColumnDef::new(Channel::Nudge).integer().not_null().default(0))
                    .col(ColumnDef::new(Channel::WebhookId).big_integer())
                    .col(ColumnDef::new(Channel::WebhookToken).string())
                    .col(ColumnDef::new(Channel::Paused).boolean().not_null().default(false))
                    .col(ColumnDef::new(Channel::PausedUntil).date_time())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_channel_guild")
                    .from(Channel::Table, Channel::GuildId)
                    .to(Guild::Table, Guild::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(User::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(User::Id).big_integer().not_null().primary_key())
                    .col(ColumnDef::new(User::DmChannel).big_integer().not_null())
                    .col(
                        ColumnDef::new(User::Timezone)
                            .custom(Timezone::Type)
                            .not_null()
                            .default("UTC"),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_user_channel")
                    .from(User::Table, User::DmChannel)
                    .to(Channel::Table, Channel::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Reminder::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Reminder::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Reminder::Uid).string().char_len(64).not_null())
                    .col(
                        ColumnDef::new(Reminder::Name)
                            .string()
                            .char_len(24)
                            .default("Reminder")
                            .not_null(),
                    )
                    .col(ColumnDef::new(Reminder::ChannelId).big_integer().not_null())
                    .col(ColumnDef::new(Reminder::UtcTime).date_time().not_null())
                    .col(
                        ColumnDef::new(Reminder::Timezone)
                            .custom(Timezone::Type)
                            .not_null()
                            .default("UTC"),
                    )
                    .col(ColumnDef::new(Reminder::IntervalSeconds).integer())
                    .col(ColumnDef::new(Reminder::IntervalMonths).integer())
                    .col(ColumnDef::new(Reminder::Enabled).boolean().not_null().default(false))
                    .col(ColumnDef::new(Reminder::Expires).date_time())
                    .col(ColumnDef::new(Reminder::Username).string_len(32))
                    .col(ColumnDef::new(Reminder::Avatar).string_len(512))
                    .col(ColumnDef::new(Reminder::Content).string_len(2000))
                    .col(ColumnDef::new(Reminder::Tts).boolean().not_null().default(false))
                    .col(ColumnDef::new(Reminder::Attachment).binary_len(8 * 1024 * 1024))
                    .col(ColumnDef::new(Reminder::AttachmentName).string_len(260))
                    .col(ColumnDef::new(Reminder::EmbedTitle).string_len(256))
                    .col(ColumnDef::new(Reminder::EmbedDescription).string_len(4096))
                    .col(ColumnDef::new(Reminder::EmbedImageUrl).string_len(500))
                    .col(ColumnDef::new(Reminder::EmbedThumbnailUrl).string_len(500))
                    .col(ColumnDef::new(Reminder::EmbedFooter).string_len(2048))
                    .col(ColumnDef::new(Reminder::EmbedFooterUrl).string_len(500))
                    .col(ColumnDef::new(Reminder::EmbedAuthor).string_len(256))
                    .col(ColumnDef::new(Reminder::EmbedAuthorUrl).string_len(500))
                    .col(ColumnDef::new(Reminder::EmbedColor).integer())
                    .col(ColumnDef::new(Reminder::EmbedFields).json())
                    .col(ColumnDef::new(Reminder::SetAt).date_time().not_null().default("NOW()"))
                    .col(ColumnDef::new(Reminder::SetBy).big_integer().not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_reminder_channel")
                    .from(Reminder::Table, Reminder::ChannelId)
                    .to(Channel::Table, Channel::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_reminder_user")
                    .from(Reminder::Table, Reminder::SetBy)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(ReminderTemplate::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ReminderTemplate::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(ReminderTemplate::GuildId).big_integer().not_null())
                    .col(
                        ColumnDef::new(ReminderTemplate::Name)
                            .string()
                            .char_len(24)
                            .default("Reminder")
                            .not_null(),
                    )
                    .col(ColumnDef::new(ReminderTemplate::Username).string_len(32))
                    .col(ColumnDef::new(ReminderTemplate::Avatar).string_len(512))
                    .col(ColumnDef::new(ReminderTemplate::Content).string_len(2000))
                    .col(ColumnDef::new(ReminderTemplate::Tts).boolean().not_null().default(false))
                    .col(ColumnDef::new(ReminderTemplate::Attachment).binary_len(8 * 1024 * 1024))
                    .col(ColumnDef::new(ReminderTemplate::AttachmentName).string_len(260))
                    .col(ColumnDef::new(ReminderTemplate::EmbedTitle).string_len(256))
                    .col(ColumnDef::new(ReminderTemplate::EmbedDescription).string_len(4096))
                    .col(ColumnDef::new(ReminderTemplate::EmbedImageUrl).string_len(500))
                    .col(ColumnDef::new(ReminderTemplate::EmbedThumbnailUrl).string_len(500))
                    .col(ColumnDef::new(ReminderTemplate::EmbedFooter).string_len(2048))
                    .col(ColumnDef::new(ReminderTemplate::EmbedFooterUrl).string_len(500))
                    .col(ColumnDef::new(ReminderTemplate::EmbedAuthor).string_len(256))
                    .col(ColumnDef::new(ReminderTemplate::EmbedAuthorUrl).string_len(500))
                    .col(ColumnDef::new(ReminderTemplate::EmbedColor).integer())
                    .col(ColumnDef::new(ReminderTemplate::EmbedFields).json())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_reminder_template_guild")
                    .from(ReminderTemplate::Table, ReminderTemplate::GuildId)
                    .to(Guild::Table, Guild::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Timer::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Timer::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Timer::StartTime).date_time().not_null().default("NOW()"))
                    .col(ColumnDef::new(Timer::Name).string_len(32).not_null().default("Timer"))
                    .col(ColumnDef::new(Timer::UserId).big_integer())
                    .col(ColumnDef::new(Timer::GuildId).big_integer())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_timer_user")
                    .from(Timer::Table, Timer::UserId)
                    .to(Guild::Table, Guild::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_timer_guild")
                    .from(Timer::Table, Timer::GuildId)
                    .to(Guild::Table, Guild::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Todo::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Todo::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Todo::UserId).big_integer())
                    .col(ColumnDef::new(Todo::GuildId).big_integer())
                    .col(ColumnDef::new(Todo::ChannelId).big_integer())
                    .col(ColumnDef::new(Todo::Value).string_len(2000).not_null())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_todo_user")
                    .from(Todo::Table, Todo::UserId)
                    .to(User::Table, User::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_todo_guild")
                    .from(Todo::Table, Todo::GuildId)
                    .to(Guild::Table, Guild::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_todo_channel")
                    .from(Todo::Table, Todo::ChannelId)
                    .to(Channel::Table, Channel::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(CommandMacro::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(CommandMacro::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(CommandMacro::GuildId).big_integer().not_null())
                    .col(ColumnDef::new(CommandMacro::Name).string_len(100).not_null())
                    .col(ColumnDef::new(CommandMacro::Description).string_len(100))
                    .col(ColumnDef::new(CommandMacro::Commands).json())
                    .to_owned(),
            )
            .await?;

        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_command_macro_guild")
                    .from(CommandMacro::Table, CommandMacro::GuildId)
                    .to(Guild::Table, Guild::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_foreign_key(
                ForeignKey::drop().table(Channel::Table).name("fk_channel_guild").to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop().table(User::Table).name("fk_user_channel").to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop().table(Reminder::Table).name("fk_reminder_channel").to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop().table(Reminder::Table).name("fk_reminder_user").to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .table(ReminderTemplate::Table)
                    .name("fk_reminder_template_guild")
                    .to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop().table(Timer::Table).name("fk_timer_user").to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop().table(Timer::Table).name("fk_timer_guild").to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(ForeignKey::drop().table(Todo::Table).name("fk_todo_user").to_owned())
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop().table(Todo::Table).name("fk_todo_guild").to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop().table(Todo::Table).name("fk_todo_channel").to_owned(),
            )
            .await?;
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .table(CommandMacro::Table)
                    .name("fk_command_macro_guild")
                    .to_owned(),
            )
            .await?;

        manager.drop_table(Table::drop().table(Guild::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Channel::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(User::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Reminder::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ReminderTemplate::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Timer::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Todo::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CommandMacro::Table).to_owned()).await?;

        manager.drop_type(Type::drop().name(Timezone::Type).to_owned()).await?;

        Ok(())
    }
}
