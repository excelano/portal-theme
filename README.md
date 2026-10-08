# portal-theme

Follow the Linux desktop's light and dark setting in an egui window, through
the XDG desktop portal.

`winit` reports the system theme on Windows and macOS and reports nothing on
Linux, so an egui window there falls back to dark whatever the desktop is set
to, and the window's own titlebar can draw light in the same frame. This crate
asks the portal's `org.freedesktop.appearance` `color-scheme` once and then
follows it for as long as the window is open. On the other two platforms it
does nothing, since the toolkit is already answering and pinning a preference
would replace a live answer with a stale one.

## Using it

```rust
eframe::run_native("An application", options, Box::new(|cc| {
    portal_theme::follow(&cc.egui_ctx);
    Ok(Box::new(App::new()))
}))
```

One call where the context exists. Nothing is returned, nothing is kept by the
caller, and the thread that watches for changes ends with the process.

## Two rules

**No preference is read as light.** The portal defines 0 as no preference, 1
as prefer dark and 2 as prefer light. GNOME's Settings offers Light and Dark
and spells them `default` and `prefer-dark`, so a person choosing Light produces
0 and never 2; `prefer-light` can only be set by hand through `gsettings`.
Reading 0 as a reason to stay dark would give every GNOME user who chose Light
a dark window, which is the defect this crate exists to prevent. Light is also
what GTK does with its own default, so a window following this rule looks like
its neighbours. A value the specification does not define is light for the same
reason: it is not evidence that a person wants a dark window.

**Nothing learned is not an answer.** Where there is no session bus, no portal,
or no appearance namespace, nothing is set and egui's own fallback stands. A
machine without a portal behaves as if this crate were not there.

## What it costs

Pure Rust, no build script, no C. The portal is reached over D-Bus through
`zbus`, which an egui application on Linux already compiles for its screen
reader support, and the crate's own source is `forbid(unsafe_code)`. Off Linux
it depends on `egui` alone and compiles to an empty function.

## Licence

MIT.
