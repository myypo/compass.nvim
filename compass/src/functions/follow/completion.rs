use crate::viml::CompassArgs;

pub fn get_follow_completion(cargs: &CompassArgs) -> Vec<String> {
    let Some(first) = cargs.sub_cmds.first() else {
        return Vec::from(&["buf".to_owned()]);
    };

    match *first {
        "buf" => Vec::from(&["target=".to_owned(), "max_windows=".to_owned()]),

        _ => Vec::from(&["buf".to_owned()]),
    }
}
