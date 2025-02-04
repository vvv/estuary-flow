use crate::errors::Error;

use std::process::{ExitStatus, Stdio};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

pub const READY: &[u8] = "READY\n".as_bytes();

// Start the connector directly.
pub fn invoke_connector_direct(entrypoint: String, args: Vec<String>) -> Result<Child, Error> {
    invoke_connector(
        Stdio::piped(),
        Stdio::piped(),
        Stdio::inherit(),
        &entrypoint,
        &args,
    )
}

// Check the connector execution exit status.
// TODO: replace this function after `exit_status_error` is stable. https://github.com/rust-lang/rust/issues/84908
pub fn check_exit_status(message: &str, result: std::io::Result<ExitStatus>) -> Result<(), Error> {
    match result {
        Ok(status) => {
            if status.success() {
                tracing::info!("{} exited without error", message);
                Ok(())
            } else {
                match status.code() {
                    Some(code) => Err(Error::CommandExecutionError(format!(
                        "{} failed with code {}.",
                        message, code
                    ))),
                    None => Err(Error::CommandExecutionError(format!(
                        "{} process terminated by signal",
                        message
                    ))),
                }
            }
        }
        Err(e) => Err(e.into()),
    }
}

// Instead of starting the connector directly, `invoke_connector_delayed` starts a
// shell process that reads a first line, and then starts the connector. This is to allow
// time for us to write down configuration files for Airbyte connectors before starting them up.
// The stdin passed to delayed connector processes must start with a line that serves as a signal
// for readiness of the configuration files.
pub fn invoke_connector_delayed(entrypoint: String, args: Vec<String>) -> Result<Child, Error> {
    tracing::debug!(%entrypoint, ?args, "invoke_connector_delayed");

    let flat_args = args
        .iter()
        .map(|arg| format!("\"{arg}\""))
        .collect::<Vec<String>>()
        .join(" ");

    invoke_connector(
        Stdio::piped(),
        Stdio::piped(),
        Stdio::inherit(),
        "sh",
        &[
            "-c".to_string(),
            format!("read -r connector_proxy_dummy_var && exec {entrypoint} {flat_args}"),
        ],
    )
}

// A more flexible API for starting the connector.
pub fn invoke_connector(
    stdin: Stdio,
    stdout: Stdio,
    stderr: Stdio,
    entrypoint: &str,
    args: &[String],
) -> Result<Child, Error> {
    tracing::debug!(%entrypoint, ?args, "invoke_connector");

    Command::new(entrypoint)
        .stdin(stdin)
        .stdout(stdout)
        .stderr(stderr)
        .args(args)
        .spawn()
        .map_err(|e| e.into())
}

pub fn parse_child(mut child: Child) -> Result<(Child, ChildStdin, ChildStdout), Error> {
    let stdout = child.stdout.take().ok_or(Error::MissingIOPipe)?;
    let stdin = child.stdin.take().ok_or(Error::MissingIOPipe)?;

    Ok((child, stdin, stdout))
}
