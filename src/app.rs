use std::rc::Rc;
use sycamore::prelude::*;

mod exercise_view;

#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Screen {
    Main,
    Practice,
    ExerciseList,
    ExerciseView(usize),
}

#[component]
pub fn App() -> View {
    let screen = create_signal(Screen::Main);

    wasm_bindgen_futures::spawn_local(async {
        if let Some(json) = crate::tauri_cmd::load_exercises().await {
            if let Ok(exercises) = serde_json::from_str(&json) {
                crate::exercises::set_exercises(exercises);
            }
        }
    });

    view! {
        main(class="container") {
            (if screen.get() == Screen::Main {
                main_screen(&screen)
            } else if screen.get() == Screen::Practice {
                practice_section(&screen)
            } else if screen.get() == Screen::ExerciseList {
                exercise_section(&screen)
            } else {
                exercise_view::exercise_view(&screen)
            })
        }
    }
}

fn main_screen(screen: &Signal<Screen>) -> View {
    let s = *screen;
    let on_practice = move |_| s.set(Screen::Practice);
    let on_exercises = move |_| s.set(Screen::ExerciseList);

    view! {
        div(class="main-screen") {
            div(class="logo-area") {
                img(src="public/logo.svg", alt="Guitar Trainer")
            }
            div(class="button-area") {
                button(class="main-btn", on:click=on_practice) { "Practice" }
                button(class="main-btn", on:click=on_exercises) { "Exercises" }
            }
        }
    }
}

fn practice_section(screen: &Signal<Screen>) -> View {
    let scr = *screen;
    let on_back = move |_| scr.set(Screen::Main);

    let cats = crate::exercises::Category::all().to_vec();
    let cats_for_start = cats.clone();
    let all = Rc::new(crate::exercises::all_exercises().clone());
    let checked = create_signal(vec![false; cats.len()]);

    let any_checked = create_selector(move || checked.with(|v| v.iter().any(|&x| x)));

    let on_start = move |_| {
        let ch = checked.with(|v| v.clone());
        let mut picked: Vec<usize> = Vec::new();
        for (i, cat) in cats_for_start.iter().enumerate() {
            if i < ch.len() && ch[i] {
                let candidates: Vec<usize> = all
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.category == *cat)
                    .map(|(_, e)| e.id as usize - 1)
                    .collect();
                if !candidates.is_empty() {
                    let r = (js_sys::Math::random() * candidates.len() as f64) as usize;
                    picked.push(candidates[r]);
                }
            }
        }
        if !picked.is_empty() {
            scr.set(Screen::ExerciseView(picked[0]));
        }
    };

    let checkboxes: Vec<View> = cats
        .iter()
        .enumerate()
        .map(|(i, cat)| {
            let cb = checked;
            let cat_name = cat.name().to_owned();
            let idx = i;
            view! {
                label(class="prac-cat-label") {
                    input(
                        r#type="checkbox",
                        checked=move || cb.with(|v| v[idx]),
                        on:input=move |_| cb.update(|v| v[idx] = !v[idx])
                    )
                    span { (cat_name) }
                }
            }
        })
        .collect();

    view! {
        div(class="practice-screen") {
            h1 { "Practice" }
            p { "Select one or more categories. A random exercise will be chosen from each." }
            div(class="prac-categories") { (checkboxes) }
            div(class="prac-actions") {
                button(
                    disabled=!any_checked.get(),
                    class=if any_checked.get() { "main-btn" } else { "main-btn disabled" },
                    on:click=on_start
                ) { "Start Practice" }
                button(class="sec-btn", on:click=on_back) { "Back" }
            }
            (if !any_checked.get() {
                view! { p(class="prac-hint") { "Select at least one category above." } }
            } else {
                view! {}
            })
        }
    }
}

fn exercise_section(screen: &Signal<Screen>) -> View {
    let scr = *screen;
    let on_back = move |_| scr.set(Screen::Main);

    let grouped = Rc::new(exercise_view::grouped_exercises());
    let selected_cat = create_signal(None::<usize>);

    let cat_tabs: Vec<View> = {
        let mut tabs: Vec<View> = Vec::new();
        tabs.push(view! {
            button(
                class=if selected_cat.get().is_none() { "cat-tab active" } else { "cat-tab" },
                on:click=move |_| selected_cat.set(None)
            ) { "All" }
        });
        for (i, (cat, _)) in grouped.iter().enumerate() {
            let ci = i;
            let cat_name = cat.name().to_owned();
            tabs.push(view! {
                button(
                    class=if selected_cat.get() == Some(ci) { "cat-tab active" } else { "cat-tab" },
                    on:click=move |_| selected_cat.set(Some(ci))
                ) { (cat_name) }
            });
        }
        tabs
    };

    let ex_rows = move || {
        let si = selected_cat.get();
        let mut rows: Vec<View> = Vec::new();
        for (ci, (cat, exercises)) in grouped.iter().enumerate() {
            if si.is_some() && si != Some(ci) {
                continue;
            }
            let cat_name = cat.name().to_owned();
            let mut ex_views: Vec<View> = Vec::new();
            for ex in exercises {
                let idx = ex.id as usize - 1;
                let ex_name = ex.name.clone();
                let nav = move |_| scr.set(Screen::ExerciseView(idx));
                ex_views.push(view! {
                    button(class="ex-item", on:click=nav) { (ex_name) }
                });
            }
            rows.push(view! {
                div(class="ex-category") {
                    h2 { (cat_name) }
                    div(class="ex-list") { (ex_views) }
                }
            });
        }
        rows
    };

    view! {
        div(class="screen") {
            h1 { "Exercises" }
            button(on:click=on_back) { "Back" }
            div(class="cat-tabs") { (cat_tabs) }
            div(class="ex-browser") { (ex_rows) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_enum_variants() {
        assert!(matches!(Screen::Main, Screen::Main));
        assert!(matches!(Screen::Practice, Screen::Practice));
        assert!(matches!(Screen::ExerciseList, Screen::ExerciseList));
        assert!(matches!(Screen::ExerciseView(0), Screen::ExerciseView(_)));
    }
}
