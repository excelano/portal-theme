//! Follow the desktop's light and dark setting where the window toolkit will
//! not: on Linux, through the XDG desktop portal.
//!
//! `winit` reports the system theme on Windows and macOS and reports nothing
//! on Linux, so egui there falls back to dark whatever the desktop says. This
//! crate asks the portal's `org.freedesktop.appearance` `color-scheme` once
//! and then follows it for as long as the window is open. On the other two
//! platforms [`follow`] is empty, since pinning a preference there would
//! replace the toolkit's live answer with a stale one.
//!
//! ```no_run
//! // In eframe, from the creation closure, where the context exists.
//! fn set_up(ctx: &egui::Context) {
//!     portal_theme::follow(ctx);
//! }
//! ```
//!
//! Two rules make it behave like the desktop's other applications. **No
//! preference is read as light**: GNOME's Settings spells Light as the
//! portal's 0 and never as 2, so reading 0 as "leave it dark" would give every
//! GNOME user who chose Light a dark window, and GTK reads its own default the
//! same way. **Nothing learned is not an answer**: where there is no session
//! bus, no portal, or no appearance namespace, nothing is set and egui's own
//! fallback stands, so a machine without a portal behaves as if this crate
//! were not there.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]
#![warn(missing_docs, clippy::pedantic)]

/// Which way a `color-scheme` answer points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    /// The desktop prefers dark.
    Dark,
    /// The desktop prefers light, or states no preference.
    Light,
}

/// What the portal's `color-scheme` value means.
///
/// The specification defines 0 as no preference, 1 as prefer dark, and 2 as
/// prefer light. Zero is light here, because GNOME's Settings sends 0 for a
/// person who chose Light (`prefer-light` can only be set by hand through
/// `gsettings`), and so is any value the specification does not define: a
/// number this build does not understand is not evidence that a person wants
/// a dark window.
#[must_use]
pub fn scheme_of(color_scheme: u32) -> Scheme {
    match color_scheme {
        1 => Scheme::Dark,
        _ => Scheme::Light,
    }
}

impl From<Scheme> for egui::ThemePreference {
    fn from(scheme: Scheme) -> Self {
        match scheme {
            Scheme::Dark => Self::Dark,
            Scheme::Light => Self::Light,
        }
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use super::{scheme_of, Scheme};

    const PORTAL: &str = "org.freedesktop.portal.Desktop";
    const PATH: &str = "/org/freedesktop/portal/desktop";
    const SETTINGS: &str = "org.freedesktop.portal.Settings";
    const NAMESPACE: &str = "org.freedesktop.appearance";
    const KEY: &str = "color-scheme";

    /// The portal answers `Read` with a variant inside a variant and `ReadOne`
    /// with one; older portals lack `ReadOne` and newer ones deprecate `Read`.
    /// Rather than choose by portal version, this unwraps whatever arrived
    /// until a number falls out.
    fn number_inside(value: &zbus::zvariant::Value<'_>) -> Option<u32> {
        let mut here = value;
        for _ in 0..4 {
            match here {
                zbus::zvariant::Value::U32(n) => return Some(*n),
                zbus::zvariant::Value::Value(inner) => here = inner,
                _ => return None,
            }
        }
        None
    }

    fn proxy(
        connection: &zbus::blocking::Connection,
    ) -> Result<zbus::blocking::Proxy<'static>, zbus::Error> {
        zbus::blocking::Proxy::new(connection, PORTAL, PATH, SETTINGS)
    }

    /// Ask once. `None` where the question could not be put at all, which is
    /// not a third answer: the caller leaves the toolkit's fallback alone.
    pub fn ask() -> Option<Scheme> {
        let connection = zbus::blocking::Connection::session().ok()?;
        let proxy = proxy(&connection).ok()?;
        let value: zbus::zvariant::OwnedValue = proxy.call("Read", &(NAMESPACE, KEY)).ok()?;
        number_inside(&value).map(scheme_of)
    }

    /// Watch the setting on a thread of its own for as long as the process
    /// runs. The thread is never joined; it holds a D-Bus connection and a
    /// clone of the context, and the process exiting closes the socket.
    pub fn watch(ctx: &egui::Context, mut apply: impl FnMut(Scheme) + Send + 'static) {
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("portal-theme".to_owned())
            .spawn(move || {
                let Ok(connection) = zbus::blocking::Connection::session() else {
                    return;
                };
                let Ok(proxy) = proxy(&connection) else {
                    return;
                };
                let Ok(changes) = proxy.receive_signal("SettingChanged") else {
                    return;
                };
                for message in changes {
                    // The signal carries every setting the portal has, fonts
                    // and accent colours included, so the key is checked
                    // rather than assumed.
                    // The body is bound before it is read: deserializing
                    // inline would drop it at the end of the statement and
                    // leave the value pointing at nothing.
                    let body = message.body();
                    let Ok((namespace, key, value)) =
                        body.deserialize::<(String, String, zbus::zvariant::Value<'_>)>()
                    else {
                        continue;
                    };
                    if namespace != NAMESPACE || key != KEY {
                        continue;
                    }
                    if let Some(number) = number_inside(&value) {
                        apply(scheme_of(number));
                        // The thread has no frame of its own. Without this the
                        // new theme sits in egui's options until something
                        // else causes a repaint, which looks like the setting
                        // being ignored.
                        ctx.request_repaint();
                    }
                }
            })
            .ok();
    }
}

/// The desktop's answer right now, on Linux. `None` where no portal answered.
#[cfg(target_os = "linux")]
#[must_use]
pub fn ask() -> Option<Scheme> {
    linux::ask()
}

/// Follow the desktop's light and dark setting where the window toolkit does
/// not. Call it once where the context exists, which in eframe is the
/// creation closure.
///
/// `set_theme` rather than `Options::system_theme`, because the second is
/// what egui fills in from `winit`, and this platform's whole problem is that
/// nothing fills it in.
#[cfg(target_os = "linux")]
pub fn follow(ctx: &egui::Context) {
    if let Some(scheme) = linux::ask() {
        ctx.set_theme(egui::ThemePreference::from(scheme));
    }
    let watcher = ctx.clone();
    linux::watch(ctx, move |scheme| {
        watcher.set_theme(egui::ThemePreference::from(scheme));
    });
}

/// Nothing to do: `winit` reports the system theme on this platform and egui
/// is already following it.
#[cfg(not(target_os = "linux"))]
pub fn follow(_ctx: &egui::Context) {}

#[cfg(test)]
mod tests {
    use super::{scheme_of, Scheme};

    /// Would catch the defect this crate exists to prevent: reading the
    /// portal's *no preference*, which is what GNOME sends for Light, as a
    /// reason to leave the window dark.
    #[test]
    fn no_preference_is_light() {
        assert_eq!(scheme_of(0), Scheme::Light);
    }

    /// Would catch the mapping being inverted, which is the one way this can
    /// be wrong while appearing to work.
    #[test]
    fn one_is_dark_and_two_is_light() {
        assert_eq!(scheme_of(1), Scheme::Dark);
        assert_eq!(scheme_of(2), Scheme::Light);
    }

    /// A value outside the specification's three is light, for the reason 0
    /// is: it is not evidence that a person wants a dark window.
    #[test]
    fn a_value_the_specification_does_not_define_is_light() {
        for unknown in [3, 4, 99, u32::MAX] {
            assert_eq!(scheme_of(unknown), Scheme::Light, "{unknown}");
        }
    }

    #[test]
    fn a_scheme_becomes_the_matching_preference() {
        assert_eq!(
            egui::ThemePreference::from(Scheme::Dark),
            egui::ThemePreference::Dark
        );
        assert_eq!(
            egui::ThemePreference::from(Scheme::Light),
            egui::ThemePreference::Light
        );
    }
}
