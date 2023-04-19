use sea_orm::DatabaseConnection;

pub async fn serve(_db: DatabaseConnection) -> anyhow::Result<()> {
    Ok(())
}
