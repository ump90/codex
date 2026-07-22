use std::borrow::Cow;
use std::collections::HashMap;

use sqlx::AssertSqlSafe;
use sqlx::SqlSafeStr;
use sqlx::SqlitePool;
use sqlx::migrate::Migration;
use sqlx::migrate::Migrator;

pub(crate) static STATE_MIGRATOR: Migrator = sqlx::migrate!("./migrations");
pub(crate) static LOGS_MIGRATOR: Migrator = sqlx::migrate!("./logs_migrations");
pub(crate) static GOALS_MIGRATOR: Migrator = sqlx::migrate!("./goals_migrations");
pub(crate) static MEMORIES_MIGRATOR: Migrator = sqlx::migrate!("./memory_migrations");
pub(crate) static THREAD_HISTORY_MIGRATOR: Migrator = sqlx::migrate!("./thread_history_migrations");

/// Allow an older Codex binary to open a database that has already been
/// migrated by a newer binary running in parallel.
///
/// We intentionally ignore applied migration versions that are newer than the
/// embedded migration set. Known migration versions are still validated by
/// checksum, so this only relaxes the "database is ahead of me" case.
fn runtime_migrator(base: &'static Migrator) -> Migrator {
    Migrator {
        migrations: Cow::Borrowed(base.migrations.as_ref()),
        ignore_missing: true,
        locking: base.locking,
        no_tx: base.no_tx,
        table_name: base.table_name.clone(),
        create_schemas: base.create_schemas.clone(),
    }
}

pub(crate) fn runtime_state_migrator() -> Migrator {
    runtime_migrator(&STATE_MIGRATOR)
}

pub(crate) fn runtime_logs_migrator() -> Migrator {
    runtime_migrator(&LOGS_MIGRATOR)
}

pub(crate) fn runtime_goals_migrator() -> Migrator {
    runtime_migrator(&GOALS_MIGRATOR)
}

pub(crate) fn runtime_memories_migrator() -> Migrator {
    runtime_migrator(&MEMORIES_MIGRATOR)
}

// The paginated history projector will call this when it takes ownership of opening the database.
#[allow(dead_code)]
pub(crate) fn runtime_thread_history_migrator() -> Migrator {
    runtime_migrator(&THREAD_HISTORY_MIGRATOR)
}

pub(crate) async fn repair_legacy_recency_migration_version(
    pool: &SqlitePool,
    migrator: &Migrator,
) -> anyhow::Result<()> {
    let Some(recency_migration) = migrator
        .migrations
        .iter()
        .find(|migration| migration.version == 39)
    else {
        return Ok(());
    };
    if !migrations_table_exists(pool).await? {
        return Ok(());
    }

    for checksum in current_and_line_ending_checksums(recency_migration) {
        let legacy_recency_needs_repair = sqlx::query_scalar::<_, i64>(
            r#"
SELECT 1
FROM _sqlx_migrations
WHERE version = ?
  AND checksum = ?
  AND NOT EXISTS (
      SELECT 1 FROM _sqlx_migrations WHERE version = ?
  )
        "#,
        )
        .bind(38_i64)
        .bind(&checksum)
        .bind(recency_migration.version)
        .fetch_optional(pool)
        .await?
        .is_some();
        if !legacy_recency_needs_repair {
            continue;
        }

        sqlx::query(
            r#"
UPDATE _sqlx_migrations
SET version = ?, description = ?
WHERE version = ?
  AND checksum = ?
  AND NOT EXISTS (
      SELECT 1 FROM _sqlx_migrations WHERE version = ?
  )
        "#,
        )
        .bind(recency_migration.version)
        .bind(recency_migration.description.as_ref())
        .bind(38_i64)
        .bind(checksum)
        .bind(recency_migration.version)
        .execute(pool)
        .await?;
        break;
    }
    Ok(())
}

pub(crate) async fn line_ending_compatible_migrator(
    pool: &SqlitePool,
    migrator: &Migrator,
) -> anyhow::Result<Migrator> {
    let mut migrations = migrator.iter().cloned().collect::<Vec<_>>();
    if !migrations_table_exists(pool).await? {
        return Ok(migrator_with_migrations(migrator, migrations));
    }

    let applied_checksums = sqlx::query_as::<_, (i64, Vec<u8>)>(
        "SELECT version, checksum FROM _sqlx_migrations WHERE success = 1",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .collect::<HashMap<_, _>>();

    for migration in &mut migrations {
        let Some(applied_checksum) = applied_checksums.get(&migration.version) else {
            continue;
        };
        if migration.checksum.as_ref() == applied_checksum.as_slice() {
            continue;
        }
        if line_ending_checksum_variants(migration)
            .iter()
            .any(|checksum| checksum.as_slice() == applied_checksum.as_slice())
        {
            migration.checksum = Cow::Owned(applied_checksum.clone());
        }
    }

    Ok(migrator_with_migrations(migrator, migrations))
}

async fn migrations_table_exists(pool: &SqlitePool) -> anyhow::Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_optional(pool)
    .await?
    .is_some())
}

fn current_and_line_ending_checksums(migration: &Migration) -> Vec<Vec<u8>> {
    let mut checksums = vec![migration.checksum.to_vec()];
    checksums.extend(line_ending_checksum_variants(migration));
    checksums
}

fn migrator_with_migrations(base: &Migrator, migrations: Vec<Migration>) -> Migrator {
    Migrator {
        migrations: Cow::Owned(migrations),
        ignore_missing: base.ignore_missing,
        locking: base.locking,
        no_tx: base.no_tx,
        table_name: base.table_name.clone(),
        create_schemas: base.create_schemas.clone(),
    }
}

fn line_ending_checksum_variants(migration: &Migration) -> Vec<Vec<u8>> {
    let sql = migration.sql.as_str();
    let lf_sql = sql.replace("\r\n", "\n");
    let crlf_sql = lf_sql.replace('\n', "\r\n");

    [lf_sql, crlf_sql]
        .into_iter()
        .filter(|variant_sql| variant_sql.as_str() != sql)
        .map(|variant_sql| {
            Migration::new(
                migration.version,
                migration.description.clone(),
                migration.migration_type,
                AssertSqlSafe(variant_sql).into_sql_str(),
                migration.no_tx,
            )
            .checksum
            .to_vec()
        })
        .filter(|checksum| checksum.as_slice() != migration.checksum.as_ref())
        .collect()
}

#[cfg(test)]
#[path = "migrations_tests.rs"]
mod tests;
