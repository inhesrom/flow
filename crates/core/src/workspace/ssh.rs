use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;

use anyhow::{bail, Result};
use protocol::SshTarget;
use tokio::process::Command;

/// Builds a `Command` that either runs locally or tunnels through SSH.
///
/// For local execution (`ssh == None`), returns `Command::new(program)` with `current_dir(cwd)`.
/// For SSH execution, returns `ssh` with ControlMaster args that runs `cd <cwd> && <program> <args>`.
pub fn build_command(
    ssh: Option<&SshTarget>,
    cwd: &Path,
    program: &str,
    args: &[&str],
) -> Command {
    match ssh {
        None => {
            let mut cmd = Command::new(program);
            cmd.args(args).current_dir(cwd);
            cmd
        }
        Some(target) => {
            let mut cmd = Command::new("ssh");
            append_ssh_args(&mut cmd, target);
            cmd.arg(ssh_destination(target));

            let remote_cmd = format!(
                "cd {} && {} {}",
                shell_quote(&cwd.display().to_string()),
                shell_quote(program),
                args.iter()
                    .map(|a| shell_quote(a))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            cmd.arg(remote_cmd);
            cmd
        }
    }
}

/// Validates SSH connectivity and that the remote path exists.
pub async fn validate_ssh_connection(target: &SshTarget, path: &Path) -> Result<()> {
    let mut cmd = Command::new("ssh");
    append_ssh_args(&mut cmd, target);
    cmd.arg("-o").arg("BatchMode=yes");
    cmd.arg("-o").arg("ConnectTimeout=5");
    cmd.arg(ssh_destination(target));
    cmd.arg(format!(
        "test -d {}",
        shell_quote(&path.display().to_string())
    ));

    let out = cmd.output().await?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!(
            "SSH connection to {} failed or path {} does not exist: {}",
            ssh_destination(target),
            path.display(),
            stderr.trim()
        );
    }
    Ok(())
}

/// Single-quote wraps a string for safe use in remote shell commands.
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Returns the `user@host` or just `host` SSH destination string.
pub fn ssh_destination(target: &SshTarget) -> String {
    match &target.user {
        Some(user) => format!("{}@{}", user, target.host),
        None => target.host.clone(),
    }
}

/// Returns a deterministic control socket path for ControlMaster multiplexing.
fn control_socket_path(target: &SshTarget) -> String {
    let mut hasher = DefaultHasher::new();
    target.host.hash(&mut hasher);
    target.user.hash(&mut hasher);
    target.port.hash(&mut hasher);
    let hash = hasher.finish();
    format!("/tmp/anvl-ssh-{:x}", hash)
}

/// Appends common SSH arguments (ControlMaster, port) to a command.
fn append_ssh_args(cmd: &mut Command, target: &SshTarget) {
    if let Some(port) = target.port {
        cmd.arg("-p").arg(port.to_string());
    }
    let socket = control_socket_path(target);
    cmd.arg("-o").arg("ControlMaster=auto");
    cmd.arg("-o").arg(format!("ControlPath={}", socket));
    cmd.arg("-o").arg("ControlPersist=600");
}

/// Delimiter used to separate output sections in a batched SSH command.
pub const BATCH_DELIM: &str = "---ANVL_BATCH_DELIM---";

/// Builds a single SSH `Command` that runs multiple shell commands on the remote,
/// separated by `BATCH_DELIM` markers so the caller can split the combined stdout.
pub fn build_batch_command(target: &SshTarget, cwd: &Path, commands: &[String]) -> Command {
    let joined = commands
        .iter()
        .map(|c| format!("{{ {}; }}", c))
        .collect::<Vec<_>>()
        .join(&format!(" ; echo '{}' ; ", BATCH_DELIM));

    let remote_cmd = format!(
        "cd {} && {{ {}; }}",
        shell_quote(&cwd.display().to_string()),
        joined
    );

    let mut cmd = Command::new("ssh");
    append_ssh_args(&mut cmd, target);
    cmd.arg(ssh_destination(target));
    cmd.arg(remote_cmd);
    cmd
}

/// Builds SSH args as a Vec<String> for use with CommandBuilder (terminals).
pub fn ssh_args_for_terminal(target: &SshTarget, cwd: &Path) -> Vec<String> {
    let mut args = vec!["ssh".to_string(), "-t".to_string()];
    if let Some(port) = target.port {
        args.push("-p".to_string());
        args.push(port.to_string());
    }
    let socket = control_socket_path(target);
    args.push("-o".to_string());
    args.push("ControlMaster=auto".to_string());
    args.push("-o".to_string());
    args.push(format!("ControlPath={}", socket));
    args.push("-o".to_string());
    args.push("ControlPersist=600".to_string());
    args.push(ssh_destination(target));
    args.push(format!(
        "cd {} && exec $SHELL -l",
        shell_quote(&cwd.display().to_string())
    ));
    args
}
