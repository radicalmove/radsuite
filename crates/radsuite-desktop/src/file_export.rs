use std::{fs, io, path::Path};

pub fn copy_local_file(source_path: &Path, destination_path: &Path) -> io::Result<()> {
    fs::copy(source_path, destination_path).map(|_| ())
}

pub fn write_local_text_file(destination_path: &Path, contents: &str) -> io::Result<()> {
    fs::write(destination_path, contents)
}
