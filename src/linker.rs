use std::{path::Path, process::{Command, ExitStatus}};

#[derive(Debug)]
pub(crate) enum LinkerError {
    ErrorStatus(ExitStatus),
}

pub(crate) fn link(asm_path : &Path, exe_out_path : &Path) -> Result<(), LinkerError> {
    let mut cmd = Command::new("gcc");
    cmd.arg(asm_path).arg("-o").arg(exe_out_path);
    let status = cmd.spawn().unwrap().wait().unwrap();
    if !status.success(){
        return Err(LinkerError::ErrorStatus(status));
    }
    Ok(())
}