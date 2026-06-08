//! review-agent
//!
//! Reads every `agents/*.md` file — each describing one git repo and an
//! instruction to perform on it — and for each one: clones/updates the repo,
//! runs Claude Code headless with the instruction, then (if anything changed)
//! commits, pushes a fresh branch, and opens a pull request via `gh`.
//!
//! Agent file format (markdown with YAML-ish frontmatter):
//!
//!   ---
//!   repo: git@github.com:owner/name.git   # required
//!   title: My PR title                    # optional (PR title + commit msg)
//!   branch_prefix: audit                  # optional (default: "agent")
//!   ---
//!   <the instruction Claude should perform on the repo>
//!
//! Shells out to `git`, `claude`, and `gh` — no third-party crates.
//!
//! Identity: if `GH_TOKEN` (or `GITHUB_TOKEN`) is set, the agent pushes over
//! token-authenticated HTTPS and opens the PR as that token's account (e.g. a
//! bot), and commits are attributed to it. With no token it falls back to the
//! repo URL as given (e.g. SSH) and your ambient `gh`/git identity.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

type R<T> = Result<T, Box<dyn Error>>;

/// File Claude writes its human-readable PR description to (first line = title,
/// remainder = body). Pulled out of the tree so the PR holds only the fix.
const PR_DESC_FILE: &str = "PR_DESCRIPTION.md";

struct AgentSpec {
    file: String,
    repo: String,
    title: Option<String>,
    branch_prefix: String,
    instruction: String,
}

fn log(msg: &str) {
    println!("\x1b[1;34m[agent]\x1b[0m {msg}");
}

/// Run a command, streaming output to the terminal; error on non-zero exit.
fn run(program: &str, args: &[&str], dir: &Path) -> R<()> {
    run_masked(program, args, dir, None)
}

/// Like `run`, but redact `secret` (e.g. a token embedded in a URL) from the
/// logged command line so it never lands in logs/journal.
fn run_masked(program: &str, args: &[&str], dir: &Path, secret: Option<&str>) -> R<()> {
    let mut shown = args.join(" ");
    if let Some(s) = secret {
        if !s.is_empty() {
            shown = shown.replace(s, "***");
        }
    }
    log(&format!("$ {program} {shown}"));
    let status = Command::new(program).args(args).current_dir(dir).status()?;
    if !status.success() {
        return Err(format!("`{program}` exited with {status}").into());
    }
    Ok(())
}

/// Run a command and capture trimmed stdout.
fn capture(program: &str, args: &[&str], dir: &Path) -> R<String> {
    let out = Command::new(program).args(args).current_dir(dir).output()?;
    if !out.status.success() {
        return Err(format!(
            "`{program} {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )
        .into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// True if `program --version` (or given args) succeeds.
fn ok(program: &str, args: &[&str]) -> bool {
    Command::new(program)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Derive a checkout directory name from a repo URL (last path segment, no .git).
fn repo_name(repo: &str) -> String {
    repo.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git")
        .to_string()
}

/// The bot token, if one is provided via the environment (non-empty).
fn env_token() -> Option<String> {
    for key in ["GH_TOKEN", "GITHUB_TOKEN"] {
        if let Ok(v) = std::env::var(key) {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}

/// Extract `owner/name` from an SSH or HTTPS GitHub URL, if it is one.
fn github_slug(repo: &str) -> Option<String> {
    let s = repo.trim().trim_end_matches('/').trim_end_matches(".git");
    let rest = s
        .strip_prefix("git@github.com:")
        .or_else(|| s.strip_prefix("https://github.com/"))
        .or_else(|| s.strip_prefix("http://github.com/"))
        .or_else(|| s.rsplit_once("@github.com/").map(|(_, r)| r))?;
    let mut parts = rest.trim_matches('/').splitn(3, '/');
    let owner = parts.next().filter(|s| !s.is_empty())?;
    let name = parts.next().filter(|s| !s.is_empty())?;
    Some(format!("{owner}/{name}"))
}

/// URL git should use for network ops. With a token on a GitHub repo, embed it
/// for HTTPS auth as the bot; otherwise the repo URL as given.
fn network_url(repo: &str, token: Option<&str>) -> String {
    match (token, github_slug(repo)) {
        (Some(t), Some(slug)) => format!("https://x-access-token:{t}@github.com/{slug}.git"),
        _ => repo.to_string(),
    }
}

/// The token account's `(name, noreply-email)`, for commit attribution.
/// Asks `gh` (which honors the token) who it is.
fn bot_identity(dir: &Path) -> Option<(String, String)> {
    let login = capture("gh", &["api", "user", "--jq", ".login"], dir).ok()?;
    let id = capture("gh", &["api", "user", "--jq", ".id"], dir).ok()?;
    if login.is_empty() {
        return None;
    }
    Some((login.clone(), format!("{id}+{login}@users.noreply.github.com")))
}

fn parse_spec(path: &Path) -> R<AgentSpec> {
    let text = fs::read_to_string(path)?;
    let file = path.file_name().unwrap_or_default().to_string_lossy().into_owned();

    let mut repo = None;
    let mut title = None;
    let mut branch_prefix = "agent".to_string();
    let body;

    if let Some(rest) = text.strip_prefix("---\n") {
        if let Some(end) = rest.find("\n---") {
            for line in rest[..end].lines() {
                if let Some((k, v)) = line.split_once(':') {
                    match k.trim() {
                        "repo" => repo = Some(v.trim().to_string()),
                        "title" => title = Some(v.trim().to_string()),
                        "branch_prefix" => branch_prefix = v.trim().to_string(),
                        _ => {}
                    }
                }
            }
            body = rest[end + 4..].trim().to_string();
        } else {
            body = text.trim().to_string();
        }
    } else {
        body = text.trim().to_string();
    }

    let repo = repo.ok_or_else(|| format!("{file}: missing `repo:` in frontmatter"))?;
    if body.is_empty() {
        return Err(format!("{file}: empty instruction body").into());
    }
    Ok(AgentSpec { file, repo, title, branch_prefix, instruction: body })
}

/// Process one agent. Returns the PR URL, or None if the repo was unchanged.
fn run_agent(path: &Path, checkout_root: &Path) -> R<Option<String>> {
    let spec = parse_spec(path)?;
    log(&format!("=== {} → {} ===", spec.file, spec.repo));

    let token = env_token();
    let net_url = network_url(&spec.repo, token.as_deref());
    let slug = github_slug(&spec.repo);

    let dir = checkout_root.join(repo_name(&spec.repo));
    let timestamp = capture("date", &["+%Y%m%d-%H%M%S"], checkout_root)?;
    let branch = format!("{}/{}", spec.branch_prefix, timestamp);

    if !dir.join(".git").is_dir() {
        log(&format!("Cloning into {}", dir.display()));
        run_masked(
            "git",
            &["clone", &net_url, &dir.to_string_lossy()],
            checkout_root,
            token.as_deref(),
        )?;
    }
    // Normalize origin to the network URL every run, so a checkout cloned under
    // a different identity (e.g. an old SSH origin) starts using the token too.
    run_masked("git", &["remote", "set-url", "origin", &net_url], &dir, token.as_deref())?;

    let base = {
        let info = capture("git", &["remote", "show", "origin"], &dir)?;
        info.lines()
            .find_map(|l| l.trim().strip_prefix("HEAD branch:"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "main".to_string())
    };

    log(&format!("Syncing with origin/{base}, branching {branch}"));
    run("git", &["fetch", "origin", "--prune"], &dir)?;
    run("git", &["checkout", &base], &dir)?;
    run("git", &["reset", "--hard", &format!("origin/{base}")], &dir)?;
    run("git", &["checkout", "-b", &branch], &dir)?;

    log("Running Claude headless — may take a few minutes…");
    run(
        "claude",
        &[
            "-p", &spec.instruction,
            "--permission-mode", "acceptEdits",
            "--allowedTools", "Read Grep Glob Bash Write Edit",
            "--output-format", "text",
            "--settings", "{\"includeCoAuthoredBy\": false}",
        ],
        &dir,
    )?;

    // Take Claude's PR description out of the working tree before measuring the
    // diff, so the PR contains only the actual fix.
    let (title, body) = read_pr_description(&dir, &spec, &timestamp);

    if capture("git", &["status", "--porcelain"], &dir)?.is_empty() {
        return Ok(None);
    }

    // Attribute the commit to the bot when running with a token; otherwise keep
    // the ambient git identity.
    let (name, email) = match token.as_deref() {
        Some(_) => bot_identity(&dir)
            .unwrap_or_else(|| ("Review Agent".to_string(), "agent@localhost".to_string())),
        None => (
            "Review Agent".to_string(),
            capture("git", &["config", "user.email"], &dir)
                .unwrap_or_else(|_| "agent@localhost".to_string()),
        ),
    };

    log("Committing & pushing");
    run("git", &["add", "-A"], &dir)?;
    run(
        "git",
        &[
            "-c", &format!("user.name={name}"),
            "-c", &format!("user.email={email}"),
            "commit", "-m", &title,
        ],
        &dir,
    )?;
    run("git", &["push", "-u", "origin", &branch], &dir)?;

    log("Opening pull request");
    let mut gh_args: Vec<&str> = vec![
        "pr", "create",
        "--base", &base,
        "--head", &branch,
        "--title", &title,
        "--body", &body,
    ];
    // Be explicit about the repo so `gh` doesn't have to parse a token-bearing
    // origin URL to figure out where the PR goes.
    if let Some(s) = &slug {
        gh_args.push("--repo");
        gh_args.push(s);
    }
    let url = capture("gh", &gh_args, &dir)?;
    Ok(Some(url))
}

/// Read and consume `PR_DESCRIPTION.md`: first non-empty line is the title, the
/// rest is the body. Falls back to the spec's title/instruction if absent.
fn read_pr_description(dir: &Path, spec: &AgentSpec, timestamp: &str) -> (String, String) {
    let default_title =
        || spec.title.clone().unwrap_or_else(|| format!("Automated change ({timestamp})"));

    let path = dir.join(PR_DESC_FILE);
    let Ok(text) = fs::read_to_string(&path) else {
        return (default_title(), spec.instruction.clone());
    };
    let _ = fs::remove_file(&path);

    let mut lines = text.trim().lines();
    let title = lines
        .next()
        .map(|l| l.trim_start_matches('#').trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(default_title);
    let body = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    let body = if body.is_empty() { title.clone() } else { body };
    (title, body)
}

fn main() {
    if let Err(e) = real_main() {
        eprintln!("\x1b[1;31m[agent] ERROR:\x1b[0m {e}");
        std::process::exit(1);
    }
}

fn real_main() -> R<()> {
    let cwd = std::env::current_dir()?;
    let agents_dir = cwd.join("agents");
    let checkout_root = cwd.join("checkout");

    for bin in ["git", "claude", "gh"] {
        if !ok(bin, &["--version"]) {
            return Err(format!("`{bin}` not found on PATH").into());
        }
    }
    if !ok("gh", &["auth", "status"]) {
        return Err("gh not authenticated — run: gh auth login".into());
    }

    let mut specs: Vec<PathBuf> = fs::read_dir(&agents_dir)
        .map_err(|e| format!("cannot read {}: {e}", agents_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();
    specs.sort();
    if specs.is_empty() {
        return Err(format!("no `.md` agent files in {}", agents_dir.display()).into());
    }

    fs::create_dir_all(&checkout_root)?;

    let mut failures = 0;
    for path in &specs {
        match run_agent(path, &checkout_root) {
            Ok(Some(url)) => log(&format!("✅ {} → {url}", path.display())),
            Ok(None) => log(&format!("• {} → no changes, skipped PR", path.display())),
            Err(e) => {
                failures += 1;
                eprintln!("\x1b[1;31m[agent] {} FAILED:\x1b[0m {e}", path.display());
            }
        }
    }

    if failures > 0 {
        return Err(format!("{failures} agent(s) failed").into());
    }
    log("All agents complete.");
    Ok(())
}
