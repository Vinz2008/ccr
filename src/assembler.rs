use std::{ffi::OsString, path::{Path, PathBuf}, process::Command};

pub(crate) fn assemble(path : &Path){
    let os_str = OsString::from_iter([path.file_stem().unwrap(), ".o".as_ref()]);
    let object_file = PathBuf::from(os_str);
    let mut cmd = Command::new("as");
    cmd.arg(path).arg("-o").arg(object_file);
    cmd.spawn().unwrap().wait().unwrap();
}