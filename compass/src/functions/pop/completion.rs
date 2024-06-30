use crate::viml::CompassArgs;

pub fn get_pop_completion(cargs: &CompassArgs) -> Vec<String> {
    let Some(first) = cargs.sub_cmds.first() else {
        return Vec::from(&["relative".to_owned()]);
    };

    match *first {
        "relative" => Vec::from(&["direction=".to_owned()]),

        _ => Vec::from(&["relative".to_owned()]),
    }
}
