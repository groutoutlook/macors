# MACORS

A cross-platform recording and playback system for keyboard and mouse macros.
All recorded macros are stored as `.toml`  and can be manually edited.

## Installation
currently unreleased - so pull code and `cargo install --path .`

also see: https://github.com/Narsil/rdevin?tab=readme-ov-file#os-caveats

## Usage
**Recording a Macro**:
   Start recording by specifying a macro name. Example:
```bash
macors rec mymacro
... do some stuff
hit ESC ESC ESC to save.
```

NOTE due to limitations of rdevin this will exiting will generate a silent panic
(so it may look like an error but it's not).

**Playing Back a Macro**:
To run a recorded macro once:
```bash
macors run mymacro
```

To run it three times consecutively:
```bash
macors run mymacro 3
```

## Settings

- **Stop Recording/Playback Keystroke(s)**:
  - A user-defined keystroke combination that, when entered during recording,
    stops the recording and is ignored in the macro.
  - The default stop sequence is \<Esc\>\<Esc\>\<Esc\>
- **Wait Strategy**:
  - **Record Actual Waits**: Records the actual time pauses between each event
    and plays them back.
  - **Constant Wait**: Uses a predefined constant wait time uniformly after each
    event.

## Files
- macros are automatically stored in: `~/.config/macors/macros/<macro-name>.toml`
- The settings file is: `~/.config/macors/settings.toml`.

## Alternatives
 - keyboard maestro (mac) 
 - autohotkey (windows) 

