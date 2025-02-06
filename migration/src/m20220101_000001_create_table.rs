use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // create tables
        manager.create_table(
            Table::create()
                .table(Servers::Table)
                .if_not_exists()
                .col(ColumnDef::new(Servers::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Servers::Ip).string().not_null())
                .col(ColumnDef::new(Servers::Port).integer().not_null())
                .col(ColumnDef::new(Servers::Version).string().not_null())
                .col(ColumnDef::new(Servers::Protocol).integer().not_null())
                .col(ColumnDef::new(Servers::MaxPlayers).integer().not_null())
                .col(ColumnDef::new(Servers::OnlinePlayers).integer().not_null())
                .col(ColumnDef::new(Servers::Auth).string().not_null())
                .col(ColumnDef::new(Servers::CreatedAt).date_time().not_null())
                .col(ColumnDef::new(Servers::UpdatedAt).date_time().not_null())
                .col(ColumnDef::new(Servers::Forge).boolean().not_null())
                .to_owned(),
        ).await?;

        manager.create_table(
            Table::create()
                .table(Descriptions::Table)
                .if_not_exists()
                .col(ColumnDef::new(Descriptions::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Descriptions::ServerId).integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_server_id")
                    .from(Descriptions::Table, Descriptions::ServerId)
                    .to(Servers::Table, Servers::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .col(ColumnDef::new(Descriptions::Text).string().not_null())
                .col(ColumnDef::new(Descriptions::Bold).boolean().not_null())
                .col(ColumnDef::new(Descriptions::Italic).boolean().not_null())
                .col(ColumnDef::new(Descriptions::Underline).boolean().not_null())
                .col(ColumnDef::new(Descriptions::Strikethrough).boolean().not_null())
                .col(ColumnDef::new(Descriptions::Obfuscated).boolean().not_null())
                .col(ColumnDef::new(Descriptions::Colour).string().not_null())
                .to_owned(),
        ).await?;

        manager.create_table(
            Table::create()
                .table(Favicons::Table)
                .if_not_exists()
                .col(ColumnDef::new(Favicons::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Favicons::ServerId).integer().not_null())
                .col(ColumnDef::new(Favicons::Png).text().not_null())
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_server_id")
                    .from(Favicons::Table, Favicons::ServerId)
                    .to(Servers::Table, Servers::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned(),
        ).await?;

        manager.create_table(
            Table::create()
                .table(Players::Table)
                .if_not_exists()
                .col(ColumnDef::new(Players::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Players::Name).string().not_null())
                .col(ColumnDef::new(Players::Uuid).string().not_null())
                .col(ColumnDef::new(Players::LastSeen).date_time().not_null())
                .col(ColumnDef::new(Players::ServerId).integer().not_null())
                .foreign_key(
                    ForeignKey::create()
                    .name("fk_server_id")
                    .from(Players::Table, Players::ServerId)
                    .to(Servers::Table, Servers::Id)
                    .on_delete(ForeignKeyAction::Cascade)
                    .on_update(ForeignKeyAction::Cascade)
                )
                .to_owned(),
        ).await?;

        manager.create_table(
            Table::create()
                .table(Ips::Table)
                .if_not_exists()
                .col(ColumnDef::new(Ips::Id).integer().not_null().auto_increment().primary_key())
                .col(ColumnDef::new(Ips::Ip).string().not_null())
                .col(ColumnDef::new(Ips::Port).integer().not_null())
                .col(ColumnDef::new(Ips::LastScanned).date_time())
                .to_owned(),
        ).await?;

        // setup indexes
        manager.create_index(
            Index::create()
            .table(Ips::Table)
            .name("idx_ips_ipport")
            .col(Ips::Ip)
            .col(Ips::Port)
            .unique()
            .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
            .table(Servers::Table)
            .name("idx_servers_ip")
            .col(Servers::Ip)
            .col(Servers::Port)
            .unique()
            .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
            .table(Descriptions::Table)
            .name("idx_descriptions_server_id")
            .col(Descriptions::ServerId)
            .col(Descriptions::Text)
            .unique()
            .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
            .table(Favicons::Table)
            .name("idx_favicons_server_id")
            .col(Favicons::ServerId)
            .unique()
            .to_owned(),
        ).await?;

        manager.create_index(
            Index::create()
            .table(Players::Table)
            .name("idx_players_server_id")
            .col(Players::ServerId)
            .col(Players::Uuid)
            .unique()
            .to_owned(),
        ).await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        
        manager
            .drop_table(Table::drop().table(Descriptions::Table).to_owned())
            .await?;
    
        manager
            .drop_table(Table::drop().table(Favicons::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Players::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Servers::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Ips::Table).to_owned())
            .await?;

        Ok(())
    }
}

/// Learn more at https://docs.rs/sea-query#iden
#[derive(Iden)]
enum Servers {
    Table,
    Id,
    Ip,
    Version,
    Protocol,
    MaxPlayers,
    OnlinePlayers,
    Auth,
    CreatedAt,
    UpdatedAt,
    Port,
    Forge,
}

#[derive(Iden)]
enum Descriptions {
    Table,
    Id,
    ServerId,
    Text,
    Bold,
    Italic,
    Underline,
    Strikethrough,
    Obfuscated,
    Colour,
}

#[derive(Iden)]
enum Favicons {
    Table,
    Id,
    ServerId,
    Png,
}

#[derive(Iden)]
enum Players {
    Table,
    Id,
    Name,
    Uuid,
    LastSeen,
    ServerId,
}

#[derive(Iden)]
enum Ips {
    Table,
    Id,
    Ip,
    Port,
    LastScanned,
}
