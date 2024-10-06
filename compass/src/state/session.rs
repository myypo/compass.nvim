mod data_session;
pub use data_session::*;

use crate::{state::Record, state::TrackList, Result};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Write},
    path::Path,
};

use anyhow::anyhow;

pub fn save_session(data: Session, path: &Path) -> Result<()> {
    let file = File::create(path).map_err(|e| anyhow!("{e}"))?;
    let mut buf = BufWriter::new(file);

    buf.write_all(&bitcode::encode(&data))
        .map_err(|e| anyhow!("{e}"))?;

    Ok(())
}

pub fn load_session(path: &Path) -> Result<TrackList<Record>> {
    let file = File::open(path).map_err(|e| anyhow!("{e}"))?;
    let mut bytes = Vec::new();
    BufReader::new(file)
        .read_to_end(&mut bytes)
        .map_err(|e| anyhow!("{e}"))?;
    let session: Session = bitcode::decode(&bytes).map_err(|e| anyhow!("{e}"))?;

    session.try_into()
}

mod tests {
    use super::*;

    use nvim_oxi::api::get_current_buf;

    use crate::state::{ChangeTypeRecord, PlaceTypeRecord};

    #[nvim_oxi::test]
    fn can_save_and_load_session() {
        let list = {
            let mut list = TrackList::default();
            for i in 0..=3 {
                list.push(
                    Record::try_new(
                        get_current_buf(),
                        PlaceTypeRecord::Change(ChangeTypeRecord::Tick(i.into())),
                        &(1, 0).into(),
                    )
                    .unwrap(),
                );
            }
            list
        };

        let mut path = std::env::temp_dir();
        path.push("compass_session_load_test_file");

        save_session(Session::try_from(&list).unwrap(), &path).unwrap();

        let got = load_session(&path).unwrap();
        let mut want = list.iter_from_future();
        for r in got.iter_from_future() {
            let w = want.next().unwrap();
            assert_eq!(r.buf, w.buf);
            assert_eq!(
                r.place_type,
                PlaceTypeRecord::Change(ChangeTypeRecord::Restored)
            );
        }
    }
}
