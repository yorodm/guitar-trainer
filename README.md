# Guitar Trainer

A desktop guitar practice application built with [Tauri](https://tauri.app) v2 and [Sycamore](https://sycamore-rs.netlify.app/) 0.9. It plays exercises using MIDI SoundFont synthesis and displays tablature, picking patterns, fingering, and note durations in real time.

## Features

- **Practice mode** — select categories, get a random exercise from each
- **Exercise browser** — filter by category, pick any exercise
- **Tablature display** — SVG-rendered 6-string tab with real-time note highlighting
- **Audio playback** — MIDI synthesis via `rustysynth` + `rodio` with a bundled SoundFont
- **Count-in** — configurable Off / First Loop / Every Loop with visual beat dots
- **BPM control** — adjustable tempo (10–300 BPM)
- **Timer** — countdown with automatic stop
- **Font scaling** — A+/A- buttons scale all UI including the SVG tablature
- **Dark mode** — full dark theme via `prefers-color-scheme`
- **Responsive** — adapts to narrow screens

## Built-in Exercises

Over 20 exercises across 10 categories:

| Category | Examples |
|---|---|
| Warmup | Spider Walk, Chromatic Scale |
| Coordination | String Skipping, Cross-String Pairs |
| Ear Training | Interval Jumps, Octave Shapes |
| Stamina | Alternate Picking Endurance |
| Rhythm | Syncopation, Triplet Patterns |
| Arpeggios | Major, Minor, Diminished |
| Modes | Ionian, Dorian, Mixolydian |
| Cross Picking | String-Crossing Patterns |
| Major Scales | Open Position, Two-Octave |
| Other | Miscellaneous |

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop shell | Tauri v2 |
| Frontend | Rust + Sycamore 0.9 (WASM) |
| Audio engine | `rustysynth` 1.3 + `rodio` 0.19 |
| SoundFont | [GeneralUser GS](https://www.schristiancollins.com/generaluser.php) (bundled) |
| Build | Trunk |
| Bundler | Tauri bundle |

## Prerequisites

- [Rust](https://rustup.rs/) (stable)
- [Trunk](https://trunkrs.dev/) (`cargo install trunk`)
- System dependencies for Tauri: see [Tauri prerequisites](https://tauri.app/v2/guides/getting-started/prerequisites)

## Getting Started

```bash
# Clone the repository
git clone https://github.com/your-username/guitar-trainer.git
cd guitar-trainer

# Run in development mode
cargo tauri dev

# Build for production
cargo tauri build
```

## Project Structure

```
guitar-trainer/
├── src/                        # Frontend (Sycamore WASM)
│   ├── main.rs                 # Entry point
│   ├── app.rs                  # Screen routing, practice & exercise browser
│   ├── app/
│   │   └── exercise_view.rs    # Exercise playback UI, scheduling engine
│   ├── exercises.rs            # Data model, JSON loading, validation
│   └── tauri_cmd.rs            # Frontend → backend IPC wrappers
├── src-tauri/                  # Backend (Tauri Rust)
│   ├── src/
│   │   ├── lib.rs              # Tauri commands, app setup
│   │   └── audio.rs            # SoundFont synth, MIDI key mapping
│   └── resources/
│       └── exercises.json      # Exercise definitions
├── styles.css                  # All styles (light + dark mode)
└── index.html                  # Trunk entry HTML
```

## Exercise Format

Exercises are defined in `src-tauri/resources/exercises.json`:

```json
{
  "id": 1,
  "name": "Spider Walk",
  "category": "Warmup",
  "description": "Ascending 1-2-3-4 chromatic pattern across all strings",
  "default_bpm": 60,
  "min_bpm": 30,
  "max_bpm": 180,
  "default_duration_secs": 60,
  "notes": [
    {"string": 6, "fret": 1, "duration": 2},
    {"string": 6, "fret": 2, "duration": 2}
  ],
  "picking": [0, 1, 0, 1],
  "fingering": [0, 1, 2, 3]
}
```

- `string`: 1 (high E) to 6 (low E)
- `duration`: 0=Whole, 1=Half, 2=Quarter, 3=Eighth, 4=Sixteenth
- `picking`: 0=Down, 1=Up (repeats cyclically)
- `fingering`: 0=Index, 1=Middle, 2=Ring, 3=Pinky, 4=Open

Exercise IDs must be contiguous starting at 1 (validated at load time).

## Adding Exercises

1. Add a new entry to `src-tauri/resources/exercises.json`
2. Assign the next sequential `id`
3. Pick an existing `category` from the enum (Warmup, Coordination, EarTraining, Stamina, Rhythm, Arpeggios, Modes, CrossPicking, MajorScales, Other)
4. The exercise is automatically available on next launch

## License

This project does not currently include a license file. Contact the author for usage terms.

---

**Disclaimer:** The UI layer of this project and portions of the codebase were developed with the assistance of AI. Code review passes were also conducted with AI support.
