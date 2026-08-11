use std::{fs, path::PathBuf};

use radsuite_desktop::{copy_local_file, write_local_text_file};
use uuid::Uuid;

#[test]
fn copy_local_file_writes_the_selected_destination() {
    let root = std::env::temp_dir().join(format!("radsuite-file-export-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create temp directory");
    let source = root.join("source.vtt");
    let destination = root.join("saved").join("lesson.vtt");
    fs::create_dir_all(destination.parent().expect("destination parent"))
        .expect("create destination directory");
    fs::write(&source, "WEBVTT\n\n00:00.000 --> 00:01.000\nHello\n").expect("write source");

    copy_local_file(&source, &destination).expect("copy local file");

    assert_eq!(
        fs::read_to_string(&destination).expect("read destination"),
        "WEBVTT\n\n00:00.000 --> 00:01.000\nHello\n"
    );
    remove_tree(&root);
}

#[test]
fn write_local_text_file_preserves_generated_html() {
    let root = std::env::temp_dir().join(format!("radsuite-text-export-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create temp directory");
    let destination = root.join("readings.html");
    let contents = "<!doctype html>\n<h1>Readings</h1>";

    write_local_text_file(&destination, contents).expect("write local text file");

    assert_eq!(
        fs::read_to_string(&destination).expect("read destination"),
        contents
    );
    remove_tree(&root);
}

fn remove_tree(path: &PathBuf) {
    let _ = fs::remove_dir_all(path);
}
