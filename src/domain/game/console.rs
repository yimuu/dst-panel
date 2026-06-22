//! Safe screen command construction for migrated game-console routes.
//!
//! Go built these commands with shell strings such as
//! `screen -S ... -X stuff "<command>\n"`. Rust keeps the same target behavior
//! but uses argv arrays through [`crate::infra::command::CommandRunner`], so cluster,
//! level, KU id, message, and console text are never interpolated into a shell.

use crate::{
    infra::command::{CommandRunner, CommandSpec},
    validation::{validate_ku_id, validate_safe_command_arg},
    web::error::{AppError, AppResult},
};

const SCREEN_PROGRAM: &str = "screen";

/// Sends a raw console command to one level.
pub async fn send_level_command(
    runner: &dyn CommandRunner,
    cluster_name: &str,
    level_name: &str,
    command: &str,
) -> AppResult<()> {
    let cluster = validate_safe_command_arg("cluster name", cluster_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    let level = validate_safe_command_arg("level name", level_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    validate_console_text(command)?;
    run_screen_command(
        runner,
        cluster.as_str(),
        level.as_str(),
        command,
        "level-command",
    )
    .await;
    Ok(())
}

/// Sends a broadcast announcement to both default shards.
pub async fn broadcast(
    runner: &dyn CommandRunner,
    cluster_name: &str,
    message: &str,
) -> AppResult<()> {
    validate_console_text(message)?;
    let escaped = lua_double_quote_escape(message);
    send_both_default_levels(
        runner,
        cluster_name,
        &format!("c_announce(\"{escaped}\")"),
        "broadcast",
    )
    .await
}

/// Sends `TheNet:Kick` to both default shards for a validated KU id.
pub async fn kick_player(
    runner: &dyn CommandRunner,
    cluster_name: &str,
    ku_id: &str,
) -> AppResult<()> {
    let ku_id = validate_ku_id(ku_id).map_err(|error| AppError::bad_request(error.to_string()))?;
    send_both_default_levels(
        runner,
        cluster_name,
        &format!("TheNet:Kick(\"{}\")", ku_id.as_str()),
        "kick-player",
    )
    .await
}

/// Sends the legacy death event command to both default shards.
pub async fn kill_player(
    runner: &dyn CommandRunner,
    cluster_name: &str,
    ku_id: &str,
) -> AppResult<()> {
    let ku_id = validate_ku_id(ku_id).map_err(|error| AppError::bad_request(error.to_string()))?;
    send_both_default_levels(
        runner,
        cluster_name,
        &format!("UserToPlayer(\"{}\"):PushEvent('death')", ku_id.as_str()),
        "kill-player",
    )
    .await
}

/// Sends the legacy respawn event command to both default shards.
pub async fn respawn_player(
    runner: &dyn CommandRunner,
    cluster_name: &str,
    ku_id: &str,
) -> AppResult<()> {
    let ku_id = validate_ku_id(ku_id).map_err(|error| AppError::bad_request(error.to_string()))?;
    send_both_default_levels(
        runner,
        cluster_name,
        &format!(
            "UserToPlayer(\"{}\"):PushEvent('respawnfromghost')",
            ku_id.as_str()
        ),
        "respawn-player",
    )
    .await
}

/// Announces and requests a rollback by day count on both default shards.
pub async fn rollback(runner: &dyn CommandRunner, cluster_name: &str, days: i64) -> AppResult<()> {
    broadcast(runner, cluster_name, &format!(":pig 正在回档{days}天")).await?;
    send_both_default_levels(
        runner,
        cluster_name,
        &format!("c_rollback({days})"),
        "rollback",
    )
    .await
}

/// Announces and requests world regeneration on both default shards.
pub async fn regenerate_world(runner: &dyn CommandRunner, cluster_name: &str) -> AppResult<()> {
    broadcast(runner, cluster_name, ":pig 即将重置世界！！！").await?;
    send_both_default_levels(
        runner,
        cluster_name,
        "c_regenerateworld()",
        "regenerate-world",
    )
    .await
}

/// Sends the master console route command.
pub async fn master_console(
    runner: &dyn CommandRunner,
    cluster_name: &str,
    command: &str,
) -> AppResult<()> {
    send_level_command(runner, cluster_name, "Master", command).await
}

/// Sends the caves console route command.
///
/// Go's Unix and Windows implementations both dispatch `/api/game/caves/console`
/// to the Master screen session. Preserve that compatibility quirk in this
/// migration slice; changing the target should be a deliberate behavior fix.
pub async fn caves_console(
    runner: &dyn CommandRunner,
    cluster_name: &str,
    command: &str,
) -> AppResult<()> {
    send_level_command(runner, cluster_name, "Master", command).await
}

async fn send_both_default_levels(
    runner: &dyn CommandRunner,
    cluster_name: &str,
    command: &str,
    action: &'static str,
) -> AppResult<()> {
    let cluster = validate_safe_command_arg("cluster name", cluster_name)
        .map_err(|error| AppError::bad_request(error.to_string()))?;
    validate_console_text(command)?;
    for level in ["Master", "Caves"] {
        run_screen_command(runner, cluster.as_str(), level, command, action).await;
    }
    Ok(())
}

async fn run_screen_command(
    runner: &dyn CommandRunner,
    cluster_name: &str,
    level_name: &str,
    command: &str,
    action: &'static str,
) {
    let spec = screen_command_spec(cluster_name, level_name, command);
    tracing::info!(
        action,
        cluster_name,
        level_name,
        command_len = command.len(),
        "sending DST console command"
    );
    match runner.run(spec).await {
        Ok(output) => {
            tracing::debug!(
                action,
                cluster_name,
                level_name,
                status_code = ?output.status_code,
                stdout_len = output.stdout.len(),
                stderr_len = output.stderr.len(),
                "DST console command runner returned"
            );
        }
        Err(error) => {
            tracing::warn!(
                action,
                cluster_name,
                level_name,
                error = %error,
                "DST console command runner failed; preserving Go success response"
            );
        }
    }
}

pub(crate) fn screen_command_spec(
    cluster_name: &str,
    level_name: &str,
    command: &str,
) -> CommandSpec {
    CommandSpec::new(SCREEN_PROGRAM)
        .arg("-S")
        .arg(screen_session_key(cluster_name, level_name))
        .arg("-p")
        .arg("0")
        .arg("-X")
        .arg("stuff")
        .arg(format!("{command}\n"))
}

pub(crate) fn screen_session_key(cluster_name: &str, level_name: &str) -> String {
    format!("DST_8level_{cluster_name}_{level_name}")
}

fn validate_console_text(value: &str) -> AppResult<()> {
    if value.contains('\0') {
        return Err(AppError::bad_request(
            "console text contains unsafe characters",
        ));
    }
    if value.len() > 8192 {
        return Err(AppError::bad_request("console text is too long"));
    }
    Ok(())
}

fn lua_double_quote_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_command_spec_uses_argv_without_shell() {
        let spec = screen_command_spec("ClusterA", "Master", "c_save()");

        assert_eq!(spec.program(), "screen");
        assert_eq!(
            spec.args(),
            [
                "-S",
                "DST_8level_ClusterA_Master",
                "-p",
                "0",
                "-X",
                "stuff",
                "c_save()\n"
            ]
        );
    }

    #[test]
    fn lua_double_quote_escape_keeps_message_inside_string_literal() {
        assert_eq!(
            lua_double_quote_escape("hi\");c_shutdown();--"),
            "hi\\\");c_shutdown();--"
        );
    }
}
