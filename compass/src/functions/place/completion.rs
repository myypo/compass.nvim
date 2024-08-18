use crate::viml::CompassArgs;

pub fn get_place_completion(cargs: &CompassArgs) -> Vec<String> {
    let Some(_) = cargs.sub_cmds.first() else {
        return Vec::from(&["change".to_owned()]);
    };

    Vec::from(&["change".to_owned()])
}
