use gloo_timers::callback::{Interval, Timeout};
use std::cell::RefCell;
use std::rc::Rc;
use sycamore::prelude::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::MouseEvent;

use crate::app::Screen;
use crate::exercises;

#[derive(Clone, Copy, PartialEq, Eq)]
enum CountInMode {
    None,
    FirstLoop,
    EveryLoop,
}

struct PlaybackState {
    timeout: Option<Timeout>,
    timer: Option<Interval>,
}

impl PlaybackState {
    const fn new() -> Self {
        Self {
            timeout: None,
            timer: None,
        }
    }

    fn cancel(&mut self) {
        if let Some(t) = self.timeout.take() {
            t.cancel();
        }
        if let Some(t) = self.timer.take() {
            t.cancel();
        }
    }
}

const fn note_duration_ms(duration: exercises::NoteDuration, bpm: u16) -> u32 {
    let quarter = 60_000 / bpm as u32;
    match duration {
        exercises::NoteDuration::Whole => quarter * 4,
        exercises::NoteDuration::Half => quarter * 2,
        exercises::NoteDuration::Quarter => quarter,
        exercises::NoteDuration::Eighth => quarter / 2,
        exercises::NoteDuration::Sixteenth => quarter / 4,
    }
}

#[allow(clippy::too_many_arguments)]
fn schedule_tick(
    pos: usize,
    notes: Rc<Vec<exercises::Note>>,
    bpm_sig: Signal<u16>,
    playing: Signal<bool>,
    state: Rc<RefCell<PlaybackState>>,
    cur_pos: Signal<Option<usize>>,
    count_in_mode: Signal<CountInMode>,
    remaining_secs: Signal<u32>,
) {
    if remaining_secs.get() == 0 {
        playing.set(false);
        cur_pos.set(None);
        return;
    }

    if pos >= notes.len() {
        let ci = count_in_mode.get();
        if ci == CountInMode::EveryLoop {
            let n2 = notes;
            let s2 = state.clone();
            schedule_countin(0, bpm_sig, playing, state, cur_pos, Rc::new(move || {
                schedule_tick(0, n2.clone(), bpm_sig, playing, s2.clone(), cur_pos, count_in_mode, remaining_secs);
            }));
        } else {
            schedule_tick(0, notes, bpm_sig, playing, state, cur_pos, count_in_mode, remaining_secs);
        }
        return;
    }

    cur_pos.set(Some(pos));

    let bpm = bpm_sig.get();
    let note = &notes[pos];
    let dur_ms = note_duration_ms(note.duration, bpm);

    spawn_local({
        let s = note.string;
        let f = note.fret;
        async move {
            crate::tauri_cmd::play_note(s, f).await;
        }
    });

    let state2 = state.clone();
    let notes2 = notes.clone();
    let timeout = Timeout::new(dur_ms, move || {
        if !playing.get() {
            return;
        }

        spawn_local({
            let s = notes2[pos].string;
            let f = notes2[pos].fret;
            async move {
                crate::tauri_cmd::stop_note(s, f).await;
            }
        });

        let next_pos = pos + 1;
        schedule_tick(next_pos, notes2, bpm_sig, playing, state2, cur_pos, count_in_mode, remaining_secs);
    });

    state.borrow_mut().timeout = Some(timeout);
}

fn schedule_countin(
    beat: u8,
    bpm_sig: Signal<u16>,
    playing: Signal<bool>,
    state: Rc<RefCell<PlaybackState>>,
    cur_pos: Signal<Option<usize>>,
    on_done: Rc<dyn Fn()>,
) {
    if beat >= 4 || !playing.get() {
        (on_done)();
        return;
    }

    cur_pos.set(None);
    let bpm = bpm_sig.get();
    let quarter_ms = 60_000 / bpm as u32;

    let state2 = state.clone();
    let timeout = Timeout::new(quarter_ms, move || {
        schedule_countin(beat + 1, bpm_sig, playing, state2, cur_pos, on_done);
    });

    state.borrow_mut().timeout = Some(timeout);
}

pub fn grouped_exercises() -> Vec<(exercises::Category, Vec<exercises::Exercise>)> {
    let all = exercises::all_exercises();
    let mut groups: Vec<(exercises::Category, Vec<exercises::Exercise>)> = Vec::new();
    for cat in exercises::Category::all() {
        let list: Vec<_> = all.iter().filter(|e| e.category == *cat).cloned().collect();
        if !list.is_empty() {
            groups.push((*cat, list));
        }
    }
    groups
}

const DUR_SYMS: &[&str] = &["\u{1D15D}", "\u{266C}", "\u{2669}", "\u{266A}", "\u{266B}"];

const fn pick_sym(d: exercises::PickingDirection) -> &'static str {
    match d {
        exercises::PickingDirection::Down => "\u{2193}",
        exercises::PickingDirection::Up => "\u{2191}",
    }
}

const fn finger_label(f: exercises::Finger) -> &'static str {
    match f {
        exercises::Finger::Index => "1",
        exercises::Finger::Middle => "2",
        exercises::Finger::Ring => "3",
        exercises::Finger::Pinky => "4",
        exercises::Finger::Open => "O",
    }
}

fn fmt_time(secs: u32) -> String {
    let m = secs / 60;
    let s = secs % 60;
    format!("{:02}:{:02}", m, s)
}

pub fn exercise_view(screen: &Signal<Screen>) -> View {
    let Screen::ExerciseView(idx) = screen.get() else {
        unreachable!()
    };
    let all = exercises::all_exercises();
    let exercise = all[idx].clone();

    let bpm = create_signal(exercise.default_bpm);
    let playing = create_signal(false);
    let cur_pos = create_signal(None);
    let pb_state = Rc::new(RefCell::new(PlaybackState::new()));
    let pb_notes = Rc::new(exercise.notes.clone());
    let font_size = create_signal::<u16>(16);
    let count_in_mode = create_signal(CountInMode::FirstLoop);
    let remaining_secs = create_signal(exercise.default_duration_secs);
    let default_duration = exercise.default_duration_secs;

    let on_play: Rc<dyn Fn(MouseEvent)> = Rc::new({
        let pn = pb_notes.clone();
        let pb = pb_state.clone();
        move |_: web_sys::MouseEvent| {
            pb.borrow_mut().cancel();
            remaining_secs.set(default_duration);
            playing.set(true);

            let interval = Interval::new(1000, {
                let pb2 = pb.clone();
                let p2 = playing;
                move || {
                    remaining_secs.update(|v| {
                        if *v > 0 {
                            *v -= 1;
                        }
                    });
                    if remaining_secs.get() == 0 {
                        pb2.borrow_mut().cancel();
                        spawn_local(async { crate::tauri_cmd::stop_all_notes().await; });
                        p2.set(false);
                    }
                }
            });
            pb.borrow_mut().timer = Some(interval);

            let ci_mode = count_in_mode.get();
            if ci_mode == CountInMode::None {
                schedule_tick(0, pn.clone(), bpm, playing, pb.clone(), cur_pos, count_in_mode, remaining_secs);
            } else {
                let pn2 = pn.clone();
                let pb2 = pb.clone();
                schedule_countin(0, bpm, playing, pb.clone(), cur_pos, Rc::new(move || {
                    schedule_tick(0, pn2.clone(), bpm, playing, pb2.clone(), cur_pos, count_in_mode, remaining_secs);
                }));
            }
        }
    });

    let on_stop: Rc<dyn Fn(MouseEvent)> = Rc::new({
        let pb = pb_state.clone();
        move |_: web_sys::MouseEvent| {
            pb.borrow_mut().cancel();
            spawn_local(async { crate::tauri_cmd::stop_all_notes().await; });
            cur_pos.set(None);
            remaining_secs.set(default_duration);
            playing.set(false);
        }
    });

    let on_back = {
        let s = *screen;
        let pb = pb_state;
        move |_| {
            if playing.get() {
                pb.borrow_mut().cancel();
                spawn_local(async { crate::tauri_cmd::stop_all_notes().await; });
                cur_pos.set(None);
                remaining_secs.set(default_duration);
                playing.set(false);
            }
            s.set(Screen::ExerciseList)
        }
    };

    let name = exercise.name.clone();
    let note_count = exercise.notes.len();
    let picking = exercise.picking.clone();
    let fingering = exercise.fingering;
    let viewbox_w = 28 + note_count * 36 + 10;
    let fret_notes_row = pb_notes.clone();
    let fret_notes_tab = pb_notes;

    let on_font_inc = move |_| font_size.update(|v| *v = (*v + 2).min(32));
    let on_font_dec = move |_| font_size.update(|v| *v = v.saturating_sub(2).max(10));

    view! {
        div(class="ev") {
            div(class="ev-header") {
                button(on:click=on_back) { "\u{2190} Back" }
                h1 { (name) }
                span(class="spacer")
                div(class="font-ctrl") {
                    button(on:click=on_font_dec) { "A-" }
                    span { (font_size.get()) }
                    button(on:click=on_font_inc) { "A+" }
                }
            }
            div(
                class="ev-body",
                style=format!("font-size: {}px", font_size.get())
            ) {
                div(class="ev-section") {
                    h2 { "Note Duration" }
                    div(class="dur-row") { ({
                        let notes = fret_notes_row.clone();
                        let n = note_count;
                        move || (0..n).map(|i| {
                            let d = DUR_SYMS[notes[i].duration as usize];
                            view! { span(class=if Some(i) == cur_pos.get() { "dur-sym active" } else { "dur-sym" }) { (d) } }
                        }).collect::<Vec<View>>()
                    }) }
                }
                div(class="ev-section") {
                    h2 { "Tablature" }
                    div(class="tab-wrap") {
                        svg(
                            xmlns="http://www.w3.org/2000/svg",
                            viewBox=format!("0 0 {} 170", viewbox_w),
                            width="100%",
                            height="170",
                            class="tab-svg"
                        ) {
                            (tab_lines(note_count))
                            ({
                                let notes = fret_notes_tab.clone();
                                move || {
                                    let p = cur_pos.get();
                                    const STRING_YS: [i32; 6] = [28, 52, 76, 100, 124, 148];
                                    notes.iter().enumerate().map(|(i, note)| {
                                        let x = 28 + i * 36 + 12;
                                        let sy = STRING_YS[(note.string - 1) as usize];
                                        let fret = format!("{}", note.fret);
                                        let cls = if Some(i) == p { "fret active" } else { "fret" };
                                        view! {
                                            text(
                                                x=format!("{}", x), y=format!("{}", sy + 5),
                                                class=cls, font-size="13", font-weight="bold",
                                                font-family="monospace", text-anchor="middle"
                                            ) { (fret) }
                                        }
                                    }).collect::<Vec<View>>()
                                }
                            })
                        }
                    }
                }
                div(class="ev-section") {
                    h2 { "Picking Pattern" }
                    div(class="pick-row") { ({
                        let pi = picking.clone();
                        let n = note_count;
                        move || (0..n).map(|i| {
                            let sym = pick_sym(pi[i % pi.len()]);
                            view! { span(class=if Some(i) == cur_pos.get() { "pick-sym active" } else { "pick-sym" }) { (sym) } }
                        }).collect::<Vec<View>>()
                    }) }
                }
                div(class="ev-section") {
                    h2 { "Fingering" }
                    div(class="finger-row") { ({
                        let fi = fingering.clone();
                        let n = note_count;
                        move || (0..n).map(|i| {
                            let label = finger_label(fi[i % fi.len()]);
                            view! { span(class=if Some(i) == cur_pos.get() { "finger-sym active" } else { "finger-sym" }) { (label) } }
                        }).collect::<Vec<View>>()
                    }) }
                }
                div(class="ev-section controls-row") {
                    (bpm_control(bpm))
                    (count_in_control(count_in_mode))
                    (timer_display(remaining_secs))
                }
                div(class="ev-section controls-row") {
                    div(class="ctrl transport-wrap") {
                        (if playing.get() {
                            let cb = on_stop.clone();
                            view! { button(class="transport stop", on:click=move |ev| cb(ev)) { "\u{25A0}" } }
                        } else {
                            let cb = on_play.clone();
                            view! { button(class="transport play", on:click=move |ev| cb(ev)) { "\u{25B6}" } }
                        })
                    }
                }
            }
        }
    }
}

fn tab_lines(note_count: usize) -> Vec<View> {
    let col_w = 36;
    let start_x = 28;
    const STRING_YS: [i32; 6] = [28, 52, 76, 100, 124, 148];
    let label_names = ["e", "B", "G", "D", "A", "E"];

    let mut els = Vec::new();
    for (i, sy) in STRING_YS.iter().enumerate() {
        let x2 = start_x + note_count * col_w;
        els.push(view! {
            line(
                x1=format!("{}", start_x), y1=format!("{}", sy),
                x2=format!("{}", x2), y2=format!("{}", sy),
                stroke="#999", stroke-width="1"
            )
        });
        els.push(view! {
            text(
                x="4", y=format!("{}", sy + 5),
                fill="#888", font-size="12", font-weight="bold",
                font-family="monospace"
            ) { (label_names[i]) }
        });
    }
    els
}

fn bpm_control(bpm: Signal<u16>) -> View {
    let on_inc = move |_| bpm.update(|v| *v = (*v + 5).min(300));
    let on_dec = move |_| bpm.update(|v| *v = v.saturating_sub(5).max(10));

    view! {
        div(class="ctrl") {
            label { "BPM" }
            div(class="ctrl-group") {
                button(class="ctrl-btn", on:click=on_dec) { "-" }
                span(class="ctrl-val") { (bpm.get()) }
                button(class="ctrl-btn", on:click=on_inc) { "+" }
            }
        }
    }
}

fn count_in_control(mode: Signal<CountInMode>) -> View {
    let set_none = move |_| mode.set(CountInMode::None);
    let set_first = move |_| mode.set(CountInMode::FirstLoop);
    let set_every = move |_| mode.set(CountInMode::EveryLoop);

    view! {
        div(class="ctrl") {
            label { "Count-in" }
            div(class="ctrl-group") {
                button(
                    class=if mode.get() == CountInMode::None { "ci-btn active" } else { "ci-btn" },
                    on:click=set_none
                ) { "Off" }
                button(
                    class=if mode.get() == CountInMode::FirstLoop { "ci-btn active" } else { "ci-btn" },
                    on:click=set_first
                ) { "1st" }
                button(
                    class=if mode.get() == CountInMode::EveryLoop { "ci-btn active" } else { "ci-btn" },
                    on:click=set_every
                ) { "All" }
            }
        }
    }
}

fn timer_display(remaining: Signal<u32>) -> View {
    view! {
        div(class="ctrl") {
            label { "Time" }
            span(class="timer-val") { (fmt_time(remaining.get())) }
        }
    }
}
