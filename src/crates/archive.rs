use failure::Error;
use flate2::read::GzDecoder;
use remove_dir_all::remove_dir_all;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use tar::Archive;

pub fn unpack(src: &Path, dest: &Path) -> Result<(), Error> {
    let mut file = File::open(src)?;
    let mut tar = Archive::new(GzDecoder::new(BufReader::new(&mut file)));

    if let Err(err) = unpack_without_first_dir(&mut tar, dest) {
        let _ = remove_dir_all(dest);
        Err(err)
    } else {
        Ok(())
    }
}

fn unpack_without_first_dir<R: Read>(archive: &mut Archive<R>, path: &Path) -> Result<(), Error> {
    let entries = archive.entries()?;
    for entry in entries {
        let mut entry = entry?;
        let relpath = {
            let path = entry.path();
            let path = path?;
            path.into_owned()
        };
        let mut components = relpath.components();
        // Throw away the first path component
        components.next();
        let full_path = path.join(&components.as_path());
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        entry.unpack(&full_path)?;
    }

    Ok(())
}
