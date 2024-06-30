use crate::viml::CompassArgs;

pub fn get_place_completion(cargs: &CompassArgs) -> Vec<String> {
    let Some(first) = cargs.sub_cmds.first() else {
        return Vec::from(&["change".to_owned()]);
    };

    match *first {
        "change" => Vec::from(&["try_update=".to_owned()]),

        _ => Vec::from(&["change".to_owned()]),
    }
}
