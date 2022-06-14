use std::{io, process};

const DEFAULT_VERSION: &str = "???";

fn main() {
    let cmd_result = process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output();

    let git_hash = handle_cmd_git_output(cmd_result);

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
}

fn handle_cmd_git_output(input: io::Result<process::Output>) -> String {
    match input {
        Err(err) => {
            println!("cargo:warning=unable to execute git command... {:?}", err);
        },
        Ok(output) => {
            let is_exit_code_ok = output.status.success();
            if !is_exit_code_ok {
                println!("cargo:warning=got non-0 exit code... {}", output.status.to_string());
            }

            let mut had_err = false;
            if let Some(err_out) = String::from_utf8(output.stderr).ok() {
                if !err_out.trim().is_empty() {
                    println!("cargo:warning={}", err_out);
                    had_err = true;
                }
            }

            if is_exit_code_ok && !had_err {
                if let Some(out) = String::from_utf8(output.stdout).ok() {
                    let cleaned_up = out.trim().to_ascii_lowercase();
                    if cleaned_up.is_empty() {
                        println!("cargo:warning=no version returned from git??");
                    } else {
                        return cleaned_up;
                    }
                } else {
                    println!("cargo:warning=failed to interpret stdout, not utf8?");
                }
            }
        }
    }

    println!("cargo:warning=unable to determine version... using default version '{}'", DEFAULT_VERSION);
    DEFAULT_VERSION.to_string()
}
