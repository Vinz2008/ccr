use std::{path::Path, process::Command};

pub(crate) fn link(asm_path : &Path, exe_out_path : &Path){
    let mut cmd = Command::new("gcc");
    cmd.arg(asm_path).arg("-o").arg(exe_out_path);
    cmd.spawn().unwrap().wait().unwrap();
}