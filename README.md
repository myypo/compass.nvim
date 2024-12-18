<!-- panvimdoc-ignore-start -->
<h2 align="center">🧭 compass.nvim</h2>
<video width="95%" src="https://github.com/user-attachments/assets/c19e1344-db86-428d-94ca-25c424d0cf3e"></video>
<video width="95%" src="https://github.com/user-attachments/assets/15ede53c-aadc-457b-b6fc-47b134f88ee2"></video>

## ❓ What is this and why?

Compass is an attempt to expand on the concept of `:h changelist` to allow:

- Chronological navigation between changes located in different files.
- Persisting change marks between vim sessions.
- Providing optional visual feedback and other ergonomics.
- Using recorded changes akin to builtin `:h mark`.

The end goal of the plugin is to improve codebase navigation in a way
that does not require proactive considerations on which location will be of interest later.

## 🧪 State of the plugin

The plugin can be considered **experimental**,
currently there are certainly both bugs and conceptual problems.

I will be grateful for both feedback/ideas
on how to make the project align better with its core goals in [discussions](https://github.com/myypo/compass.nvim/discussions)
and bug reports/feature requests filed as [issues](https://github.com/myypo/compass.nvim/issues)

<!-- panvimdoc-ignore-end -->

## 🎯 Requirements

- **Linux x86_64**, **MacOS ARM64/x86_64** or **Windows x86_64**.

- Neovim **v0.10.0** or **nightly (might be broken)**. Earlier versions are unlikely to work.

- For installation with make script: **GNU Make** and **curl** installed and in PATH OR **Rust toolchain** to compile yourself.

- Recommended: for persisting marks across sessions a **session manager plugin** is required that saves buffer data.

## 🔌 Installation

### [lazy.nvim](https://github.com/folke/lazy.nvim)

```lua
{
    "myypo/compass.nvim",
    build = "make",
    event = "BufReadPost",
    opts = {},
}
```

The plugin uses [nvim-oxi](https://github.com/noib3/nvim-oxi)
to make it possible to use rust instead of lua/vimscript.

The provided above installation snippet will download a pre-built by GitHub action library which should
work out of the box, but you can also build it yourself, provided, you have Rust toolchain installed.
To do so just change `build = "make"` to `build = "make build"`.

## ⌨️ Keymaps

Example useful keymaps for **Lazy.nvim**

```lua
keys = {
    -- Choose between previous locations in all buffers to where to jump to
    { "<C-f>",   "<Cmd>Compass open<CR>" },

    -- Choose between locations in the current buffer to jump to
    { "<C-S-f>", "<Cmd>Compass follow buf<CR>" },

    -- Move back to the most recent created/updated location
    { "<C-g>",   "<Cmd>Compass goto relative direction=back<CR>" },
    -- Move forward to the next location
    { "<C-S-g>", "<Cmd>Compass goto relative direction=forward<CR>" },

    -- Like goto but also deletes that plugin mark
    { "<C-p>",   "<Cmd>Compass pop relative direction=back<CR>" },
    { "<C-S-p>", "<Cmd>Compass pop relative direction=forward<CR>" },

    -- Manually place a change mark that works the same way as automatically put ones
    { "<C-m>",   "<Cmd>Compass place change<CR>" },
},

```

## ⚙️ Configuration

Default configuration:

```lua
{
    -- Options for customizing the UI that allows to preview and jump to one of the plugin marks
    picker = {
        max_windows = 6, -- Limit of windows to be opened, must be an even number

        -- List of keys to be used to jump to a plugin mark
        -- Length of this table must be equal or bigger than `max_windows`
        jump_keys = {
            -- First key is for previewing more jump marks in the file
            -- Second key is to immediately jump to this mark
            { "j", "J" },
            { "f", "F" },
            { "k", "K" },
            { "d", "D" },
            { "l", "L" },
            { "s", "S" },
            { ";", ":" },
            { "a", "A" },
            { "h", "H" },
            { "g", "G" },
        },

        filename = {
            enable = true, -- Whether to preview filename of the buffer next to the picker hint
            depth = 2, -- How many components of the path to show, `2` only shows the filename and the name of the parent directory
        },
    },

    -- Options for the plugin marks
    marks = {
        -- When applicable, how close an old plugin mark has to be to the newly placed one
        -- for the old one to be moved to the new position instead of actually creating a new separate mark
        -- Absence of a defined value means that the condition will always evaluate to false
        update_range = {
            lines = {
                single_max_distance = 10, -- If closer than this number of lines update the existing mark
                -- If closer than this number of lines AND `columns.combined_max_distance` is closer
                -- than its respective number of columns update the existing mark
                combined_max_distance = 25,
            },
            columns = {
                single_max_distance = nil, -- If closer than this number of columns update the existing mark
                combined_max_distance = 25,
            },
        },

        -- Which signs to use for different mark types relatively to the current position
        -- You can find more Unicode options online, and, if using a patched nerdfont, here: https://www.nerdfonts.com/cheat-sheet
        signs = {
            past = "◀",
            close_past = "◀",
            future = "▶",
            close_future = "▶",
        },
    },

    -- Customization of the tracker automatically placing plugin marks on file edits etc.
    tracker = {
        -- How often to perform background actions specified in milliseconds
        -- Might end up waiting for longer than the provided value
        debounce_milliseconds = {
            run = 200, -- Change checking interval
            maintenance = 500, -- Consistency enforcing interval
            -- How long to wait before activating a freshly placed mark
            -- Inactive marks are not visualized and can't be jumped back to,
            -- but still can be jump forward to and by using goto or follow commands
            activate = 3000,
        },
        -- Files matching the following glob patterns will never be tracked
        ignored_patterns = {
			"**/.git/**",
        },
    },

    -- Plugin state persistence options
    persistence = {
        enable = true, -- Whether to persist the plugin state on the disk
        path = nil, -- Absolute path to where to persist the state, by default it assumes the default neovim share path
        interval_milliseconds = 3000, -- How often to write the plugin state to disk
    },

    -- Weights and options for the frecency algorithm
    -- When appropriate tries to prioritize showing most used and most recently used plugin marks, for example, in a picker UI
    -- NOTE: the default numbers are pretty random and I am not sure how to proceed with the feature overall
    frecency = {
        time_bucket = {
            -- This table can be of any length
            thresholds = {
                { hours = 4,  weight = 100 },
                { hours = 14, weight = 70 },
                { hours = 31, weight = 50 },
                { hours = 90, weight = 30 },
		    },
            fallback = 10, -- Default weight when a value is older than the biggest `hours` in `thresholds`
        },
        -- Weights for different types of interaction with the mark
        visit_type = {
            create = 50,
            update = 100,
            relative_goto = 50,
            absolute_goto = 100,
        },
        -- Interactions that happen earlier than the cooldown won't be taken into account when calculating marks' weights
        cooldown_seconds = 60,
    },
}

```

## 🎨 Highlights

The plugin defines highlights that can be overwritten by colorschemes or manually with: `vim.api.nvim_set_hl`

<details>
    <summary>Marks down the stack</summary>

<table style="text-align: center;">
<td><b>Highlight</b></td> <td><b>Default</b> </td>

<tr>
<td>CompassRecordPast</td>
<td>

```
NONE
```

</td>

<tr>
<td>CompassRecordPastSign</td>
<td>

```
guibg=#303030 gui=bold
```

</td>

<tr>
<td>CompassRecordClosePast</td>
<td>

```
guifg=DarkRed guibg=#303030 gui=bold
```

</td>

<tr>
<td>CompassRecordClosePastSign</td>
<td>

```
guifg=DarkRed gui=bold
```

</td>

</table>

</details>

<details>
    <summary>Marks up the stack</summary>

<table style="text-align: center;">
<td><b>Highlight</b></td> <td><b>Default</b> </td>

<tr>
<td>CompassRecordFuture</td>
<td>

```
NONE
```

</td>

<tr>
<td>CompassRecordFutureSign</td>
<td>

```
guibg=#303030 gui=bold
```

</td>

<tr>
<td>CompassRecordCloseFuture</td>
<td>

```
guifg=DarkCyan guibg=#303030 gui=bold
```

</td>

<tr>
<td>CompassRecordCloseFutureSign</td>
<td>

```
guifg=DarkCyan gui=bold
```

</td>

</table>

</details>

<details>
    <summary>Picker window for `open` and `follow` commands</summary>

<table style="text-align: center;">
<td><b>Highlight</b></td> <td><b>Default</b> </td>

<tr>
<td>CompassHintOpen</td>
<td>

```
guifg=black guibg=DarkYellow gui=bold
```

</td>

<tr>
<td>CompassHintOpenPath</td>
<td>

```
guifg=DarkYellow gui=bold
```

</td>

<tr>
<td>CompassHintFollow</td>
<td>

```
guifg=black guibg=DarkYellow gui=bold
```

</td>

<tr>
<td>CompassHintFollowPath</td>
<td>

```
guifg=DarkYellow gui=bold
```

</td>

</table>

</details>
