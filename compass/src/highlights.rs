use crate::ui::{grid::GridLayout, record_mark::RecordMarkTime};
use crate::Result;

use nvim_oxi::api::command;
use serde::Deserialize;
use typed_builder::TypedBuilder;

#[derive(Deserialize, TypedBuilder)]
pub struct OptsHighlight<'a> {
    #[builder(default, setter(strip_option))]
    fg: Option<&'a str>,
    #[builder(default, setter(strip_option))]
    bg: Option<&'a str>,
    #[builder(default, setter(strip_option))]
    gui: Option<&'a str>,
}

#[derive(Deserialize)]
struct RecordHighlight<'a> {
    #[serde(borrow)]
    mark: OptsHighlight<'a>,
    #[serde(borrow)]
    sign: OptsHighlight<'a>,
}

#[derive(Deserialize)]
pub struct RecordHighlightList<'a> {
    #[serde(borrow)]
    past: RecordHighlight<'a>,
    #[serde(borrow)]
    close_past: RecordHighlight<'a>,
    #[serde(borrow)]
    future: RecordHighlight<'a>,
    #[serde(borrow)]
    close_future: RecordHighlight<'a>,
}

pub struct RecordHighlightNames {
    pub mark: &'static str,
    pub sign: &'static str,
}

impl<'a> RecordHighlightList<'a> {
    pub fn record_hl_names(typ: RecordMarkTime) -> RecordHighlightNames {
        match typ {
            RecordMarkTime::Past => RecordHighlightNames {
                mark: "CompassRecordPast",
                sign: "CompassRecordPastSign",
            },
            RecordMarkTime::PastClose => RecordHighlightNames {
                mark: "CompassRecordClosePast",
                sign: "CompassRecordClosePastSign",
            },
            RecordMarkTime::Future => RecordHighlightNames {
                mark: "CompassRecordFuture",
                sign: "CompassRecordFutureSign",
            },
            RecordMarkTime::FutureClose => RecordHighlightNames {
                mark: "CompassRecordCloseFuture",
                sign: "CompassRecordCloseFutureSign",
            },
        }
    }
}

impl<'a> Default for RecordHighlightList<'a> {
    fn default() -> Self {
        Self {
            past: RecordHighlight {
                mark: OptsHighlight::builder().bg("grey").gui("bold").build(),
                sign: OptsHighlight::builder().fg("grey").gui("bold").build(),
            },
            close_past: RecordHighlight {
                mark: OptsHighlight::builder().fg("red").gui("bold").build(),
                sign: OptsHighlight::builder().fg("red").gui("bold").build(),
            },

            future: RecordHighlight {
                mark: OptsHighlight::builder().bg("grey").gui("bold").build(),
                sign: OptsHighlight::builder().fg("grey").gui("bold").build(),
            },
            close_future: RecordHighlight {
                mark: OptsHighlight::builder().fg("blue").gui("bold").build(),
                sign: OptsHighlight::builder().fg("blue").gui("bold").build(),
            },
        }
    }
}

#[derive(Deserialize)]
struct HintHighlight<'a> {
    #[serde(borrow)]
    label: OptsHighlight<'a>,
    #[serde(borrow)]
    path: OptsHighlight<'a>,
}

#[derive(Deserialize)]
pub struct HintHighlightList<'a> {
    #[serde(borrow)]
    open: HintHighlight<'a>,
    #[serde(borrow)]
    follow: HintHighlight<'a>,
}

pub struct HintHighlightNames {
    pub mark: &'static str,
    pub path: &'static str,
}

impl<'a> HintHighlightList<'a> {
    pub fn hint_hl_names(typ: GridLayout) -> HintHighlightNames {
        match typ {
            GridLayout::Open => HintHighlightNames {
                mark: "CompassHintOpen",
                path: "CompassHintOpenPath",
            },
            GridLayout::Follow => HintHighlightNames {
                mark: "CompassHintFollow",
                path: "CompassHintFollowPath",
            },
        }
    }
}

impl<'a> Default for HintHighlightList<'a> {
    fn default() -> Self {
        Self {
            open: HintHighlight {
                label: OptsHighlight::builder()
                    .fg("black")
                    .bg("yellow")
                    .gui("bold")
                    .build(),

                path: OptsHighlight::builder().fg("yellow").gui("bold").build(),
            },
            follow: HintHighlight {
                label: OptsHighlight::builder()
                    .fg("black")
                    .bg("yellow")
                    .gui("bold")
                    .build(),

                path: OptsHighlight::builder().fg("yellow").gui("bold").build(),
            },
        }
    }
}

#[derive(Default, Deserialize)]
pub struct HighlightList<'a> {
    #[serde(borrow)]
    tracks: RecordHighlightList<'a>,
    #[serde(borrow)]
    hints: HintHighlightList<'a>,
}

pub struct IterHighlightList<'a> {
    hls: &'a HighlightList<'a>,
    index: usize,
}

impl<'a> Iterator for IterHighlightList<'a> {
    type Item = (&'static str, &'a OptsHighlight<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        let result = match self.index {
            0 => Some((
                RecordHighlightList::record_hl_names(RecordMarkTime::Past).mark,
                &self.hls.tracks.past.mark,
            )),
            1 => Some((
                RecordHighlightList::record_hl_names(RecordMarkTime::Past).sign,
                &self.hls.tracks.past.sign,
            )),
            2 => Some((
                RecordHighlightList::record_hl_names(RecordMarkTime::PastClose).mark,
                &self.hls.tracks.close_past.mark,
            )),
            3 => Some((
                RecordHighlightList::record_hl_names(RecordMarkTime::PastClose).sign,
                &self.hls.tracks.close_past.sign,
            )),
            4 => Some((
                RecordHighlightList::record_hl_names(RecordMarkTime::Future).mark,
                &self.hls.tracks.future.mark,
            )),
            5 => Some((
                RecordHighlightList::record_hl_names(RecordMarkTime::Future).sign,
                &self.hls.tracks.future.sign,
            )),
            6 => Some((
                RecordHighlightList::record_hl_names(RecordMarkTime::FutureClose).mark,
                &self.hls.tracks.close_future.mark,
            )),
            7 => Some((
                RecordHighlightList::record_hl_names(RecordMarkTime::FutureClose).sign,
                &self.hls.tracks.close_future.sign,
            )),

            8 => Some((
                HintHighlightList::hint_hl_names(GridLayout::Open).mark,
                &self.hls.hints.open.label,
            )),
            9 => Some((
                HintHighlightList::hint_hl_names(GridLayout::Open).path,
                &self.hls.hints.open.path,
            )),
            10 => Some((
                HintHighlightList::hint_hl_names(GridLayout::Follow).mark,
                &self.hls.hints.follow.label,
            )),
            11 => Some((
                HintHighlightList::hint_hl_names(GridLayout::Follow).path,
                &self.hls.hints.follow.path,
            )),

            _ => None,
        };

        self.index += 1;
        result
    }
}

impl<'a> IntoIterator for &'a HighlightList<'a> {
    type Item = (&'static str, &'a OptsHighlight<'a>);
    type IntoIter = IterHighlightList<'a>;

    fn into_iter(self) -> Self::IntoIter {
        IterHighlightList {
            hls: self,
            index: 0,
        }
    }
}

pub fn apply_highlights(hls: HighlightList) -> Result<()> {
    for (name, opts) in hls.into_iter() {
        let mut cmd = format!("hi default {}", name);

        if let Some(fg) = opts.fg {
            cmd.push_str(format!(" guifg={}", fg).as_str());
        }
        if let Some(bg) = opts.bg {
            cmd.push_str(format!(" guibg={}", bg).as_str());
        }
        if let Some(gui) = opts.gui {
            cmd.push_str(format!(" gui={}", gui).as_str());
        }

        command(cmd.as_str())?;
    }

    Ok(())
}
