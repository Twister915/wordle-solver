use std::{io, process, fmt};

const DEFAULT_VERSION: &str = "???";

fn main() {
    println!("cargo:rustc-env=GIT_HASH={}", determine_git_version());
}

fn determine_git_version() -> String {
    handle_cmd_git_output(
        process::Command::new("git")
            .args(&["rev-parse", "--short", "HEAD"])
            .output())
}

fn handle_cmd_git_output(input: io::Result<process::Output>) -> String {
    match input {
        Ok(output) => {
            // check exit status from git, should be 0
            let is_exit_code_ok = output.status.success();
            if !is_exit_code_ok {
                warning(format_args!("got non-0 exit code... {}", output.status));
            }

            // try to read stderr...
            let mut had_err = false;
            if let Some(err_out) = process_command_output_bytes("stderr", output.stderr) {
                // if there's some data in stderr, and it's non-empty, then we should print that and
                // assume the command failed...
                if !err_out.trim().is_empty() {
                    warning(format_args!("{}", err_out));
                    had_err = true;
                }
            }

            // if exit status is 0 && nothing was in stderr, then we can process stdout
            if is_exit_code_ok && !had_err {
                // try to interpret stdout
                if let Some(out) = process_command_output_bytes("stdout", output.stdout) {
                    // verify that the "cleaned up" version of the git hash is non-empty (and therefore valid)
                    let cleaned_up = out.trim().to_ascii_lowercase();
                    if cleaned_up.is_empty() {
                        warning(format_args!("no version returned from git??"))
                    } else {
                        // this branch is only reached if all checks pass
                        // we return the actual git version returned by the git command
                        return cleaned_up;
                    }
                }
            }
        }
        Err(err) => {
            warning(format_args!("unable to execute git command... {:?}", err));
        }
    }

    // if any checks fail above, then this section will be reached (because only one other return
    // is present in this function, in the "all checks passed" path)
    //
    // we write a warning & return a default version string
    warning(format_args!("unable to determine version... using default version '{}'", DEFAULT_VERSION));
    DEFAULT_VERSION.to_string()
}

fn process_command_output_bytes(name: &str, b: Vec<u8>) -> Option<String> {
    match String::from_utf8(b) {
        Ok(str) => Some(str),
        Err(err) => {
            warning(format_args!("failed to read {} output (not utf8??)... err={:?}", name, err));
            None
        }
    }
}

fn warning(args: fmt::Arguments<'_>) {
    println!("cargo:warning={}", args)
}