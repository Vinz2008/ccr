use std::{path::Path, process::Command};

pub(crate) fn link(path : &Path){
    let mut cmd = Command::new("gcc");
    cmd.arg(path);
    cmd.spawn().unwrap().wait().unwrap();
}