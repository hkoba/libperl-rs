pub use std::process::{Command, Output, ExitStatus};
pub use std::io::{Error, ErrorKind};
pub use std::result::Result;
pub use std::path::Path;

pub fn make_command(cmd_name: &str, args: &[&str]) -> Command {
    let mut cmd = Command::new(cmd_name);
    for cf in args.iter() {
        cmd.arg(cf);
    }
    cmd
}

pub fn process_command_output(out: Output) -> Result<String, Error> {
    let res = String::from_utf8_lossy(&out.stdout);

    if out.stderr.len() > 0 {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(other_error([err, res].join(": ")));
    }

    Ok(res.to_string())
}

pub fn other_error(err: String) -> Error {
    Error::new(ErrorKind::Other, err)
}

pub fn run_patch(dest_fn: &str, patch_fn: &str) -> ExitStatus {
    Command::new("patch")
        .arg("-t")
        .arg(dest_fn)
        .arg(patch_fn)
        .status()
        .unwrap()
}


