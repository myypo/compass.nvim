use crate::{
    functions::{
        follow::{get_follow, get_follow_completion},
        goto::{get_goto, get_goto_completion},
        open::{get_open, get_open_completion},
        place::{get_place, get_place_completion},
        pop::{get_pop, get_pop_completion},
        setup::get_setup,
        CommandNames,
    },
    state::Tracker,
    viml::CompassArgs,
    InputError, Result,
};
use std::{str::FromStr, sync::Mutex};

use nvim_oxi::{
    api::{
        create_user_command, notify,
        opts::{CreateCommandOpts, NotifyOpts, SetKeymapOpts},
        set_keymap,
        types::{CommandArgs, CommandComplete, CommandNArgs, LogLevel, Mode},
    },
    Dictionary, Function,
};
use strum::VariantNames;

/// Initialize the plugin as a Lua map-like table
/// This function should never return an error
pub fn init() -> Result<Dictionary> {
    let mut dict = Dictionary::new();

    let tracker = Box::leak(Box::new(Mutex::new(Tracker::default())));

    // Attaching plugin-defined functions to the lua table
    let setup = get_setup(tracker);
    dict.insert("setup", Function::<_, Result<_>>::from_fn_once(setup));

    let open = get_open(tracker);
    dict.insert("open", Function::<_, Result<_>>::from_fn(open));

    let goto = get_goto(tracker);
    dict.insert("goto", Function::<_, Result<_>>::from_fn(goto));

    let pop = get_pop(tracker);
    dict.insert("pop", Function::<_, Result<_>>::from_fn(pop));

    let follow = get_follow(tracker);
    dict.insert("follow", Function::<_, Result<_>>::from_fn(follow));

    // Setting up `Compass COMMAND` user-commands
    user_commands(tracker)?;

    Ok(dict)
}

fn user_commands(tracker: &'static Mutex<Tracker>) -> Result<()> {
    let goto = get_goto(tracker);
    let pop = get_pop(tracker);
    let open = get_open(tracker);
    let place = get_place(tracker);
    let follow = get_follow(tracker);

    let subcommands = move |ca: CommandArgs| -> Result<()> {
        let cargs =
            CompassArgs::try_from(ca.fargs.iter().map(AsRef::as_ref).collect::<Vec<&str>>())?;

        match CommandNames::from_str(cargs.main_cmd).map_err(|_| {
            InputError::FunctionArguments(
                format!("provided unknown compass subcommand: {}", cargs.main_cmd).to_owned(),
            )
        })? {
            CommandNames::Goto => goto(Some(cargs.try_into()?))?,
            CommandNames::Pop => pop(Some(cargs.try_into()?))?,
            CommandNames::Open => open(Some(cargs.try_into()?))?,
            CommandNames::Place => place(Some(cargs.try_into()?))?,
            CommandNames::Follow => follow(Some(cargs.try_into()?))?,
        };

        Ok(())
    };

    create_user_command(
        "Compass",
        Function::from_fn_mut(move |ca| {
            if let Err(e) = subcommands(ca) {
                let _ = notify(
                    &e.to_string(),
                    LogLevel::Error,
                    &NotifyOpts::builder().build(),
                );
            };
        }),
        &CreateCommandOpts::builder()
            .nargs(CommandNArgs::OneOrMore)
            .complete(cmd_completion())
            .build(),
    )?;

    plug_keymaps()?;

    Ok(())
}

fn cmd_completion() -> CommandComplete {
    CommandComplete::CustomList(Function::from(|(_, full, _): (String, String, usize)| {
        let full = full.replace("Compass", "");
        let full: Vec<&str> = full.split_whitespace().collect();

        let Ok(cargs) = TryInto::<CompassArgs>::try_into(full) else {
            return CommandNames::VARIANTS
                .iter()
                .map(|&s| s.to_owned())
                .collect::<Vec<String>>();
        };
        let Ok(cmd) = CommandNames::from_str(cargs.main_cmd) else {
            return CommandNames::VARIANTS
                .iter()
                .map(|&s| s.to_owned())
                .collect::<Vec<String>>();
        };

        match cmd {
            CommandNames::Goto => get_goto_completion(&cargs),
            CommandNames::Pop => get_pop_completion(&cargs),
            CommandNames::Open => get_open_completion(),
            CommandNames::Place => get_place_completion(&cargs),
            CommandNames::Follow => get_follow_completion(&cargs),
        }
    }))
}

fn plug_keymaps() -> Result<()> {
    set_keymap(
        Mode::Normal,
        "<Plug>(CompassOpenAll)",
        ":Compass open all<CR>",
        &SetKeymapOpts::builder().noremap(true).build(),
    )?;

    set_keymap(
        Mode::Normal,
        "<Plug>(CompassGotoBack)",
        ":Compass goto relative direction=back<CR>",
        &SetKeymapOpts::builder().noremap(true).build(),
    )?;
    set_keymap(
        Mode::Normal,
        "<Plug>(CompassGotoForward)",
        ":Compass goto relative direction=forward<CR>",
        &SetKeymapOpts::builder().noremap(true).build(),
    )?;

    set_keymap(
        Mode::Normal,
        "<Plug>(CompassPopBack)",
        ":Compass pop relative direction=back<CR>",
        &SetKeymapOpts::builder().noremap(true).build(),
    )?;
    set_keymap(
        Mode::Normal,
        "<Plug>(CompassPopForward)",
        ":Compass pop relative direction=forward<CR>",
        &SetKeymapOpts::builder().noremap(true).build(),
    )?;

    set_keymap(
        Mode::Normal,
        "<Plug>(CompassPlaceChange)",
        ":Compass place change<CR>",
        &SetKeymapOpts::builder().noremap(true).build(),
    )?;

    Ok(())
}
