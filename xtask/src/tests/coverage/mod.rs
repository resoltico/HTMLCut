use super::*;

#[cfg(unix)]
fn symlink_file(source: &Path, link: &Path) {
    std::os::unix::fs::symlink(source, link).expect("create symlink");
}

mod report_evaluation;
mod source_classification;
mod tracked_files;
