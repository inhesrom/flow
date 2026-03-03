use anyhow::Result;
use std::path::Path;
use tokio::process::Command;

use protocol::{ChangedFile, GitState};

pub async fn refresh_git(repo: &Path) -> Result<GitState> {
    let branch_out = Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .current_dir(repo)
        .output()
        .await?;

    let branch = if branch_out.status.success() {
        Some(
            String::from_utf8_lossy(&branch_out.stdout)
                .trim()
                .to_string(),
        )
        .filter(|s| !s.is_empty())
    } else {
        None
    };

    let status_out = Command::new("git")
        .arg("status")
        .arg("--porcelain=v1")
        .current_dir(repo)
        .output()
        .await?;

    let mut changed = Vec::new();
    if status_out.status.success() {
        for line in String::from_utf8_lossy(&status_out.stdout).lines() {
            if let Some(file) = parse_porcelain_line(line) {
                changed.push(file);
            }
        }
    }

    Ok(GitState { branch, changed })
}

pub async fn diff_file(repo: &Path, file: &str) -> Result<String> {
    let out = Command::new("git")
        .arg("diff")
        .arg("--")
        .arg(file)
        .current_dir(repo)
        .output()
        .await?;

    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if !text.trim().is_empty() {
        return Ok(text);
    }

    let tracked = Command::new("git")
        .arg("ls-files")
        .arg("--error-unmatch")
        .arg("--")
        .arg(file)
        .current_dir(repo)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);
    if tracked {
        return Ok(text);
    }

    let full_path = repo.join(file);
    if !full_path.exists() {
        return Ok(text);
    }
    if full_path.is_dir() {
        return Ok(format!(
            "Untracked directory: {file}\n(no file-level diff available)\n"
        ));
    }

    let bytes = std::fs::read(&full_path)?;
    if bytes.iter().any(|b| *b == 0) {
        return Ok(format!("Binary file added: {file}\n"));
    }

    let mut diff = String::new();
    diff.push_str(&format!("diff --git a/{file} b/{file}\n"));
    diff.push_str("new file mode 100644\n");
    diff.push_str("--- /dev/null\n");
    diff.push_str(&format!("+++ b/{file}\n"));
    diff.push_str("@@ -0,0 +1 @@\n");
    for line in String::from_utf8_lossy(&bytes).lines() {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    Ok(diff)
}

#[allow(dead_code)]
fn parse_porcelain_line(line: &str) -> Option<ChangedFile> {
    if line.len() < 3 {
        return None;
    }

    let status = line[..2].trim().to_string();
    let path = line[3..].trim().to_string();
    if path.is_empty() {
        return None;
    }

    Some(ChangedFile { path, status })
}
