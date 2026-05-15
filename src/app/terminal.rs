use std::{env, path::Path, process::Command};

pub(super) fn launch_in_terminal(command: &str) -> Result<(), String> {
    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let wrapped = format!("{command}; printf '\\n'; exec {}", shell_quote(&shell));
    let shell_args = shell_invocation_args(&shell, &wrapped);
    let xfce_command = format!(
        "{} {}",
        shell_quote(&shell),
        shell_args
            .iter()
            .map(|arg| shell_quote(arg))
            .collect::<Vec<_>>()
            .join(" "),
    );

    let candidates: [(&str, &[&str]); 9] = [
        ("kgx", &["--"]),
        ("ptyxis", &["--"]),
        ("gnome-terminal", &["--"]),
        ("xfce4-terminal", &["--command"]),
        ("konsole", &["-e"]),
        ("kitty", &["-e"]),
        ("alacritty", &["-e"]),
        ("foot", &["-e"]),
        ("xterm", &["-e"]),
    ];

    for (program, prefix) in candidates {
        if !command_exists(program) {
            continue;
        }

        let mut process = Command::new(program);

        if program == "xfce4-terminal" {
            process.arg("--command").arg(&xfce_command);
        } else {
            for arg in prefix {
                process.arg(arg);
            }

            process.arg(&shell);
            for arg in &shell_args {
                process.arg(arg);
            }
        }

        if process.spawn().is_ok() {
            return Ok(());
        }
    }

    Err("no supported terminal emulator found".to_string())
}

fn command_exists(name: &str) -> bool {
    if name.contains('/') {
        return Path::new(name).is_file();
    }

    let Some(paths) = env::var_os("PATH") else {
        return false;
    };

    env::split_paths(&paths).any(|directory| directory.join(name).is_file())
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\"'\"'"))
}

fn shell_invocation_args(shell: &str, command: &str) -> Vec<String> {
    match Path::new(shell).file_name().and_then(|name| name.to_str()) {
        Some("fish") => vec!["-l".to_string(), "-c".to_string(), command.to_string()],
        _ => vec!["-lc".to_string(), command.to_string()],
    }
}
