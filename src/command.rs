use std::error::Error;

use anyhow::Result;

/// Execute a command and return success/failure.
/// 
/// The `error_fn` closure is called to construct an error if the command
/// fails, allowing each caller to produce their own error type.
pub fn run_command<E>(
    bin: &str,
    args: &[&str],
    error_fn: impl Fn(String) -> E,
) -> Result<()>
where
    E: Error + Send + Sync + 'static,
{
    let output = std::process::Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| error_fn(format!("failed to run {bin}: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let msg = if stderr.is_empty() {
            format!("{bin} exited with status {}", output.status)
        } else {
            stderr
        };
        return Err(error_fn(msg).into());
    }
    Ok(())
}

/// Execute a command and return stdout.
/// 
/// The `error_fn` closure is called to construct an error if the command
/// fails or produces output that can't be converted to UTF-8, allowing each
/// caller to produce their own error type.
pub fn run_command_output<E>(
    bin: &str,
    args: &[&str],
    error_fn: impl Fn(String) -> E,
) -> Result<String>
where
    E: Error + Send + Sync + 'static,
{
    let output = std::process::Command::new(bin)
        .args(args)
        .output()
        .map_err(|e| error_fn(format!("failed to run {bin}: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let msg = if stderr.is_empty() {
            format!("{bin} exited with status {}", output.status)
        } else {
            stderr
        };
        return Err(error_fn(msg).into());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;

    #[derive(Debug)]
    struct TestError(String);

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl Error for TestError {}

    #[test]
    fn run_command_success() {
        let result = run_command("true", &[], |msg| TestError(msg));
        assert!(result.is_ok());
    }

    #[test]
    fn run_command_failure() {
        let result = run_command("false", &[], |msg| TestError(msg));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("status"));
    }

    #[test]
    fn run_command_output_success() {
        let result = run_command_output("echo", &["hello"], |msg| TestError(msg));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "hello");
    }

    #[test]
    fn run_command_output_failure() {
        let result = run_command_output("false", &[], |msg| TestError(msg));
        assert!(result.is_err());
    }

    #[test]
    fn run_command_error_includes_binary_name() {
        let result = run_command("nonexistent-binary-xyz", &[], |msg| TestError(msg));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("nonexistent-binary-xyz"));
    }
}
