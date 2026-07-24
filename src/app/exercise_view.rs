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

#[derive(Clone)]
struct PlaybackCtx {
    notes: Rc<Vec<exercises::Note>>,
    pb_state: Rc<RefCell<PlaybackState>>,
    bpm: Signal<u16>,
    playing: Signal<bool>,
    cur_pos: Signal<Option<usize>>,
    count_in_mode: Signal<CountInMode>,
    remaining_secs: Signal<u32>,
    count_beat: Signal<Option<u8>>,
    default_duration: u32,
}

const fn note_duration_ms(duration: exercises::NoteDuration, bpm: u16) -> u32 {
    let bpm = if bpm == 0 { 1 } else { bpm };
    let quarter = 60_000 / bpm as u32;
    let ms = match duration {
        exercises::NoteDuration::Whole => quarter * 4,
        exercises::NoteDuration::Half => quarter * 2,
        exercises::NoteDuration::Quarter => quarter,
        exercises::NoteDuration::Eighth => quarter / 2,
        exercises::NoteDuration::Sixteenth => quarter / 4,
    };
    if ms == 0 { 1 } else { ms }
}

fn stop_all(ctx: &PlaybackCtx) {
    ctx.pb_state.borrow_mut().cancel();
    spawn_local(async { crate::tauri_cmd::stop_all_notes().await; });
    ctx.cur_pos.set(None);
    ctx.count_beat.set(None);
    ctx.remaining_secs.set(ctx.default_duration);
    ctx.playing.set(false);
}

fn schedule_tick(pos: usize, ctx: &PlaybackCtx) {
    if ctx.remaining_secs.get() == 0 {
        ctx.playing.set(false);
        ctx.cur_pos.set(None);
        ctx.count_beat.set(None);
        return;
    }

    if pos >= ctx.notes.len() {
        if ctx.count_in_mode.get() == CountInMode::EveryLoop {
            let ctx2 = ctx.clone();
            schedule_countin(0, ctx, Rc::new(move || {
                schedule_tick(0, &ctx2);
            }));
        } else {
            schedule_tick(0, ctx);
        }
        return;
    }

    ctx.cur_pos.set(Some(pos));
    ctx.count_beat.set(None);

    let dur_ms = note_duration_ms(ctx.notes[pos].duration, ctx.bpm.get());

    spawn_local({
        let s = ctx.notes[pos].string;
        let f = ctx.notes[pos].fret;
        async move {
            crate::tauri_cmd::play_note(s, f).await;
        }
    });

    let ctx2 = ctx.clone();
    let timeout = Timeout::new(dur_ms, move || {
        if !ctx2.playing.get() {
            return;
        }

        spawn_local({
            let s = ctx2.notes[pos].string;
            let f = ctx2.notes[pos].fret;
            async move {
                crate::tauri_cmd::stop_note(s, f).await;
            }
        });

        schedule_tick(pos + 1, &ctx2);
    });

    ctx.pb_state.borrow_mut().timeout = Some(timeout);
}

fn schedule_countin(beat: u8, ctx: &PlaybackCtx, on_done: Rc<dyn Fn()>) {
    if beat >= 4 || !ctx.playing.get() {
        ctx.count_beat.set(None);
        (on_done)();
        return;
    }

    ctx.count_beat.set(Some(beat));
    ctx.cur_pos.set(None);
    let bpm = ctx.bpm.get().max(1);
    let quarter_ms = 60_000 / bpm as u32;

    let ctx2 = ctx.clone();
    let timeout = Timeout::new(quarter_ms, move || {
        schedule_countin(beat + 1, &ctx2, on_done);
    });

    ctx.pb_state.borrow_mut().timeout = Some(timeout);
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

// Whole, Beamed eighths, Quarter, Eighth, Beamed sixteenths
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

fn make_on_play(ctx: PlaybackCtx) -> Rc<dyn Fn(MouseEvent)> {
    Rc::new(move |_: MouseEvent| {
        ctx.pb_state.borrow_mut().cancel();
        ctx.remaining_secs.set(ctx.default_duration);
        ctx.playing.set(true);

        let interval = Interval::new(1000, {
            let ctx2 = ctx.clone();
            move || {
                ctx2.remaining_secs.update(|v| {
                    if *v > 0 {
                        *v -= 1;
                    }
                });
                if ctx2.remaining_secs.get() == 0 {
                    stop_all(&ctx2);
                }
            }
        });
        ctx.pb_state.borrow_mut().timer = Some(interval);

        if ctx.count_in_mode.get() == CountInMode::None {
            schedule_tick(0, &ctx);
        } else {
            let ctx2 = ctx.clone();
            schedule_countin(0, &ctx, Rc::new(move || {
                schedule_tick(0, &ctx2);
            }));
        }
    })
}

fn make_on_stop(ctx: PlaybackCtx) -> Rc<dyn Fn(MouseEvent)> {
    Rc::new(move |_: MouseEvent| {
        stop_all(&ctx);
    })
}

fn make_on_back(screen: &Signal<Screen>, ctx: PlaybackCtx) -> Rc<dyn Fn(MouseEvent)> {
    let s = *screen;
    Rc::new(move |_: MouseEvent| {
        if ctx.playing.get() {
            stop_all(&ctx);
        }
        s.set(Screen::ExerciseList)
    })
}

fn dur_row_section(notes: Rc<Vec<exercises::Note>>, note_count: usize, cur_pos: Signal<Option<usize>>) -> View {
    view! {
        div(class="ev-section") {
            h2 { "Note Duration" }
            div(class="dur-row") { ({
                let n2 = notes.clone();
                move || (0..note_count).map(|i| {
                    let d = DUR_SYMS[n2[i].duration as usize];
                    view! { span(class=if Some(i) == cur_pos.get() { "dur-sym active" } else { "dur-sym" }) { (d) } }
                }).collect::<Vec<View>>()
            }) }
        }
    }
}

fn tablature_section(notes: Rc<Vec<exercises::Note>>, note_count: usize, cur_pos: Signal<Option<usize>>, font_size: Signal<u16>) -> View {
    view! {
        div(class="ev-section") {
            h2 { "Tablature" }
            div(class="tab-wrap") {
                svg(
                    xmlns="http://www.w3.org/2000/svg",
                    viewBox=move || {
                        let s = font_size.get() as f32 / 16.0;
                        format!("0 0 {} {}", 28 + note_count * 36 + 10, (170.0 * s) as i32)
                    },
                    preserveAspectRatio="xMinYMid meet",
                    width="100%",
                    height=move || format!("{}", (170.0 * font_size.get() as f32 / 16.0) as i32),
                    class="tab-svg"
                ) {
                    ({
                        let n2 = notes.clone();
                        move || {
                            let fs_val = font_size.get();
                            let fs = fs_val as f32 / 16.0;
                            let top = (28.0 * fs) as i32;
                            let spacing = (24.0 * fs) as i32;
                            let text_off = (fs_val as i32 * 5 / 16).max(3);
                            let str_ys: [i32; 6] = [
                                top,
                                top + spacing,
                                top + spacing * 2,
                                top + spacing * 3,
                                top + spacing * 4,
                                top + spacing * 5,
                            ];
                            let label_names = ["e", "B", "G", "D", "A", "E"];
                            let p = cur_pos.get();
                            let x2 = 28 + note_count * 36;

                            let mut els: Vec<View> = Vec::new();
                            for (i, sy) in str_ys.iter().enumerate() {
                                let sy = *sy;
                                let y1 = sy.to_string();
                                let x2s = x2.to_string();
                                let y2 = sy.to_string();
                                els.push(view! {
                                    line(
                                        class="tab-line",
                                        x1="28", y1=y1,
                                        x2=x2s, y2=y2
                                    )
                                });
                                let lbl_y = (sy + text_off).to_string();
                                let lbl_font = format!("{}", (fs_val as f32 * 0.75) as u16);
                                els.push(view! {
                                    text(
                                        x="4", y=lbl_y,
                                        fill="#888", font-size=lbl_font, font-weight="bold",
                                        font-family="monospace"
                                    ) { (label_names[i]) }
                                });
                            }
                            for (i, note) in n2.iter().enumerate() {
                                let x = (28 + i * 36 + 12).to_string();
                                let sy = str_ys[(note.string - 1) as usize];
                                let y = (sy + text_off).to_string();
                                let fret = format!("{}", note.fret);
                                let cls = if Some(i) == p { "fret active" } else { "fret" };
                                let ft = format!("{}", (fs_val as f32 * 0.8125) as u16);
                                els.push(view! {
                                    text(
                                        x=x, y=y,
                                        class=cls, font-size=ft, font-weight="bold",
                                        font-family="monospace", text-anchor="middle"
                                    ) { (fret) }
                                });
                            }
                            els
                        }
                    })
                }
            }
        }
    }
}

fn picking_section(picking: Vec<exercises::PickingDirection>, note_count: usize, cur_pos: Signal<Option<usize>>) -> View {
    view! {
        div(class="ev-section") {
            h2 { "Picking Pattern" }
            div(class="pick-row") { ({
                let pi = picking.clone();
                move || (0..note_count).map(|i| {
                    let sym = pick_sym(pi[i % pi.len()]);
                    view! { span(class=if Some(i) == cur_pos.get() { "pick-sym active" } else { "pick-sym" }) { (sym) } }
                }).collect::<Vec<View>>()
            }) }
        }
    }
}

fn fingering_section(fingering: Vec<exercises::Finger>, note_count: usize, cur_pos: Signal<Option<usize>>) -> View {
    view! {
        div(class="ev-section") {
            h2 { "Fingering" }
            div(class="finger-row") { ({
                let fi = fingering.clone();
                move || (0..note_count).map(|i| {
                    let label = finger_label(fi[i % fi.len()]);
                    view! { span(class=if Some(i) == cur_pos.get() { "finger-sym active" } else { "finger-sym" }) { (label) } }
                }).collect::<Vec<View>>()
            }) }
        }
    }
}

fn header_section(
    name: String,
    font_size: Signal<u16>,
    on_back: Rc<dyn Fn(MouseEvent)>,
) -> View {
    let on_font_inc = move |_| font_size.update(|v| *v = (*v + 2).min(32));
    let on_font_dec = move |_| font_size.update(|v| *v = v.saturating_sub(2).max(10));
    let ob = on_back.clone();

    view! {
        div(class="ev-header") {
            div(class="ev-header-left") {
                button(on:click=move |ev| ob(ev)) { "\u{2190} Back" }
            }
            h1 { (name) }
            div(class="ev-header-right") {
                div(class="font-ctrl") {
                    button(on:click=on_font_dec) { "A-" }
                    span { (font_size.get()) }
                    button(on:click=on_font_inc) { "A+" }
                }
            }
        }
    }
}

fn transport_section(
    playing: Signal<bool>,
    on_play: Rc<dyn Fn(MouseEvent)>,
    on_stop: Rc<dyn Fn(MouseEvent)>,
) -> View {
    view! {
        div(class="ev-section controls-row") {
            div(class="ctrl transport-wrap") {
                (if playing.get() {
                    let s = on_stop.clone();
                    view! { button(class="transport stop", on:click=move |ev| s(ev)) { "\u{25A0}" } }
                } else {
                    let p = on_play.clone();
                    view! { button(class="transport play", on:click=move |ev| p(ev)) { "\u{25B6}" } }
                })
            }
        }
    }
}

pub fn exercise_view(screen: &Signal<Screen>) -> View {
    let Screen::ExerciseView(idx) = screen.get() else {
        unreachable!()
    };
    let all = exercises::all_exercises();
    let exercise = all[idx].clone();

    let ctx = PlaybackCtx {
        notes: Rc::new(exercise.notes.clone()),
        pb_state: Rc::new(RefCell::new(PlaybackState::new())),
        bpm: create_signal(exercise.default_bpm),
        playing: create_signal(false),
        cur_pos: create_signal(None),
        count_in_mode: create_signal(CountInMode::FirstLoop),
        remaining_secs: create_signal(exercise.default_duration_secs),
        count_beat: create_signal(None::<u8>),
        default_duration: exercise.default_duration_secs,
    };

    let font_size = create_signal::<u16>(16);

    let on_play = make_on_play(ctx.clone());
    let on_stop = make_on_stop(ctx.clone());
    let on_back = make_on_back(screen, ctx.clone());

    let note_count = exercise.notes.len();
    let notes1 = ctx.notes.clone();
    let notes2 = ctx.notes.clone();
    let cur_pos = ctx.cur_pos;
    let bpm = ctx.bpm;
    let playing = ctx.playing;
    let count_in_mode = ctx.count_in_mode;
    let count_beat = ctx.count_beat;
    let remaining_secs = ctx.remaining_secs;

    view! {
        div(class="ev") {
            (header_section(exercise.name.clone(), font_size, on_back.clone()))
            div(
                class="ev-body",
                style=move || format!("font-size: {}px", font_size.get())
            ) {
                (dur_row_section(notes1.clone(), note_count, cur_pos))
                (tablature_section(notes2.clone(), note_count, cur_pos, font_size))
                (picking_section(exercise.picking.clone(), note_count, cur_pos))
                (fingering_section(exercise.fingering.clone(), note_count, cur_pos))
                div(class="ev-section controls-row") {
                    (bpm_control(bpm))
                    (count_in_control(count_in_mode))
                    (count_in_indicator(count_beat))
                    (timer_display(remaining_secs))
                }
                (transport_section(playing, on_play.clone(), on_stop.clone()))
            }
        }
    }
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

fn count_in_indicator(beat: Signal<Option<u8>>) -> View {
    let dots = move || {
        let b = beat.get();
        (0..4).map(|i| {
            let active = b == Some(i as u8);
            view! { span(class=if active { "ci-dot active" } else { "ci-dot" }) { (i + 1) } }
        }).collect::<Vec<View>>()
    };
    view! {
        div(class="ctrl") {
            label { "Beat" }
            div(class="ci-dots") { (dots) }
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
