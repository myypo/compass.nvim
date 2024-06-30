use crate::Result;

pub fn open_tab() -> Result<()> {
    Ok(nvim_oxi::api::command("tabnew")?)
}

pub fn close_tab() -> Result<()> {
    Ok(nvim_oxi::api::command("tabclose")?)
}

mod tests {
    use super::*;
    use nvim_oxi::api::get_current_tabpage;

    #[nvim_oxi::test]
    fn can_open_tab() {
        let old_tab = get_current_tabpage().get_number().unwrap();
        assert_eq!(old_tab, 1);

        open_tab().unwrap();

        let new_tab = get_current_tabpage().get_number().unwrap();
        assert_eq!(new_tab, 2);
    }
}
