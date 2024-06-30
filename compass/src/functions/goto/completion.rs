use crate::viml::CompassArgs;

pub fn get_goto_completion(cargs: &CompassArgs) -> Vec<String> {
    let Some(first) = cargs.sub_cmds.first() else {
        return Vec::from(&["relative".to_owned(), "absolute".to_owned()]);
    };

    match *first {
        "relative" => Vec::from(&["direction=".to_owned()]),
        "absolute" => Vec::from(&["target=".to_owned()]),

        _ => Vec::from(&["relative".to_owned(), "absolute".to_owned()]),
    }
}
