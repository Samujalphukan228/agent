#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AgentMode {
    #[default]
    Base,
    God,
}

impl AgentMode {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "base" | "safe" => Some(Self::Base),
            "god" | "unrestricted" => Some(Self::God),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Base => "base",
            Self::God => "god",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::Base => "harmful actions need your approval",
            Self::God => "fully unrestricted",
        }
    }
}

pub fn needs_approval(mode: AgentMode, tool: &str, args: &serde_json::Value) -> Option<String> {
    if mode == AgentMode::God {
        return None;
    }

    match tool {
        "run_shell" => {
            let cmd = arg_str(args, "command");
            if cmd.is_empty() {
                return None;
            }
            is_harmful_shell(cmd).then(|| format!("shell: {cmd}"))
        }

        "write_file" | "edit_file" => {
            let path = arg_str(args, "path");
            approval_write(path, tool)
        }

        "create_dir" => {
            let path = arg_str(args, "path");
            approval_write(path, "create_dir")
        }

        "delete_file" => {
            let path = arg_str(args, "path");
            if path.is_empty() {
                return None;
            }
            Some(format!("delete: {path}"))
        }

        "move_file" => {
            let from = arg_str(args, "from");
            let to = arg_str(args, "to");
            if is_harmful_write(from) {
                return Some(format!("move from: {from}"));
            }
            if is_harmful_write(to) {
                return Some(format!("move to: {to}"));
            }
            if is_sensitive_path(from) || is_sensitive_path(to) {
                return Some(format!("move: {from} → {to}"));
            }
            None
        }

        "read_file" | "list_dir" | "grep" => {
            let path = arg_str(args, "path");
            approval_access(path)
        }

        "search_files" => {
            let root = arg_str(args, "root");
            approval_access(root)
        }

        "http_get" => {
            let url = arg_str(args, "url");
            if url.is_empty() {
                return None;
            }
            Some(format!("http: {url}"))
        }

        "env_info" => None,

        _ => None,
    }
}

fn arg_str<'a>(args: &'a serde_json::Value, key: &str) -> &'a str {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

fn approval_access(path: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    is_sensitive_path(path).then(|| format!("access: {path}"))
}

fn approval_write(path: &str, action: &str) -> Option<String> {
    if path.is_empty() {
        return None;
    }
    is_harmful_write(path).then(|| format!("{action}: {path}"))
}

fn is_harmful_shell(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    let patterns = [
        "rm -rf",
        "rm -fr",
        "rm -r /",
        "sudo rm",
        "dd if=",
        "mkfs.",
        "mkfs ",
        "> /dev/sd",
        "shred ",
        "chmod -r 777",
        "chmod 777 /",
        "chown -r",
        ":(){:|:&};:",
        "curl | sh",
        "curl | bash",
        "wget | sh",
        "wget | bash",
        "| sh",
        "| bash",
        "shutdown",
        "reboot",
        "poweroff",
        "init 0",
        "init 6",
        "systemctl stop",
        "systemctl disable",
        "kill -9 1",
        "killall",
        "pkill -9",
        "> /etc/",
        "tee /etc/",
        "mv / ",
        "cp /dev/zero",
        "fdisk ",
        "parted ",
        "cryptsetup ",
        "iptables -f",
        "nft flush",
    ];

    patterns.iter().any(|p| lower.contains(p))
        || (lower.contains("rm") && (lower.contains(" -r") || lower.contains(" -rf")))
        || lower.starts_with("sudo")
}

fn is_harmful_write(path: &str) -> bool {
    is_sensitive_path(path) || path.contains("..")
}

fn is_sensitive_path(path: &str) -> bool {
    let p = path.trim();
    let sensitive = [
        "/etc/passwd",
        "/etc/shadow",
        "/etc/sudoers",
        "/boot",
        "/sys",
        "/proc",
        "/dev",
        "/usr/bin",
        "/usr/sbin",
        "/bin",
        "/sbin",
        "/lib",
        "/lib64",
        "/.ssh",
        "/.gnupg",
        "/.bashrc",
        "/.profile",
        "/.config/systemd",
    ];

    sensitive.iter().any(|s| p.contains(s))
        || p.starts_with("/etc/")
        || p.starts_with("/boot/")
        || p.starts_with("/dev/")
}
