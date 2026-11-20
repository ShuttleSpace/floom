use std::path::PathBuf;

/// The dialog arms — the crate's one list of them.
///
/// The invocation below writes the list **once**, and this macro reads it
/// everywhere: it gates the arm modules, it is exactly what [`SUPPORTED`]
/// reports, it picks the arm each call jumps to, and it decides which refusal
/// exists. "An arm exists but the flag says no" is therefore not an edit this
/// crate can express — the flag and the arms are the same list.
///
/// `desktop` names a target and the module holding its real dialog; `phone` is
/// a target the crate refuses on, with a reason (see `refused`). A target in
/// neither list has no dialog at all (see `no_dialog`).
macro_rules! dialog_arms {
    (
        desktop: $($os:literal => $module:ident),+ $(,)?;
        phone: $($phone:literal),+ $(,)?;
    ) => {
        $(
            #[cfg(all(target_os = $os, not(target_env = "ohos")))]
            mod $module;
        )+

        /// Whether this target can show a file dialog.
        ///
        /// `false` means every call in this crate returns `Err` — naming the
        /// phone's missing result channel, or the target having no dialog to
        /// show. A caller reads this to hide an affordance it cannot honour,
        /// because **probing is not an option**: where this is `true`,
        /// `pick_file` puts a modal dialog on screen and waits for a human to
        /// answer it.
        ///
        /// This is the query the deleted `Capability::FILE_DIALOG` bit would
        /// have been, kept where the implementation is: `floom-platform`'s
        /// bit set is answered by the platform adapters, and no adapter can
        /// see whether this plugin is linked (see `docs/plugins.md`
        /// §8.3). The compile-time form of the same question is "is this crate
        /// in the caller's `Cargo.toml`".
        pub const SUPPORTED: bool =
            cfg!(all(any($(target_os = $os),+), not(target_env = "ohos")));

        /// The arm for `pick_file`, or this target's refusal.
        fn pick_file_arm(options: &Options) -> Result<Option<PathBuf>, String> {
            $(
                #[cfg(all(target_os = $os, not(target_env = "ohos")))]
                { return $module::pick_file(options); }
            )+
            $(
                #[cfg(target_os = $phone)]
                { let _ = options; return refused("pick_file"); }
            )+
            #[cfg(target_env = "ohos")]
            {
                let _ = options;
                return Err(no_dialog());
            }
            #[cfg(not(any($(target_os = $os),+, $(target_os = $phone),+)))]
            {
                let _ = options;
                Err(no_dialog())
            }
        }

        /// The arm for `pick_save_file`, or this target's refusal.
        fn pick_save_file_arm(options: &Options) -> Result<Option<PathBuf>, String> {
            $(
                #[cfg(all(target_os = $os, not(target_env = "ohos")))]
                { return $module::pick_save_file(options); }
            )+
            $(
                #[cfg(target_os = $phone)]
                { let _ = options; return refused("pick_save_file"); }
            )+
            #[cfg(target_env = "ohos")]
            {
                let _ = options;
                return Err(no_dialog());
            }
            #[cfg(not(any($(target_os = $os),+, $(target_os = $phone),+)))]
            {
                let _ = options;
                Err(no_dialog())
            }
        }

        /// The arm for `pick_directory`, or this target's refusal.
        fn pick_directory_arm(options: &Options) -> Result<Option<PathBuf>, String> {
            $(
                #[cfg(all(target_os = $os, not(target_env = "ohos")))]
                { return $module::pick_directory(options); }
            )+
            $(
                #[cfg(target_os = $phone)]
                { let _ = options; return refused("pick_directory"); }
            )+
            #[cfg(target_env = "ohos")]
            {
                let _ = options;
                return Err(no_dialog());
            }
            #[cfg(not(any($(target_os = $os),+, $(target_os = $phone),+)))]
            {
                let _ = options;
                Err(no_dialog())
            }
        }

        /// The phone arms' shared refusal, naming the missing plumbing.
        ///
        /// Exists only on the phone targets — the arms that call it are
        /// cfg-gated to them, and the reason is platform-specific.
        #[cfg(any($(target_os = $phone),+))]
        fn refused(call: &str) -> Result<Option<PathBuf>, String> {
            #[cfg(target_os = "android")]
            let reason = "the picker's answer arrives as an activity result \
                          (ACTION_OPEN_DOCUMENT / ACTION_CREATE_DOCUMENT), and the platform layer's \
                          NativeActivity bridge exposes no onActivityResult channel";
            #[cfg(target_os = "ios")]
            let reason = "the picker's answer arrives as a UIDocumentPickerViewController \
                          delegate callback, and presenting one needs an Objective-C delegate class \
                          — the native half this plugin shape deliberately does not carry";
            Err(format!(
                "{call}: not available on {} yet — {reason}. \
                 This is platform-layer plumbing (a result channel), not a plugin fix; \
                 desktop pickers work today",
                std::env::consts::OS
            ))
        }

        /// The refusal for a target with no dialog at all.
        #[cfg(any(
            not(any($(target_os = $os),+, $(target_os = $phone),+)),
            target_env = "ohos"
        ))]
        fn no_dialog() -> String {
            format!(
                "file dialogs need platform UI — macOS, Linux and Windows have it; \
                 this target ({}) does not",
                std::env::consts::OS
            )
        }
    };
}

dialog_arms! {
    desktop: "linux" => linux,
             "macos" => macos,
             "windows" => windows;
    phone: "android", "ios";
}

/// What the dialogs are asked, beyond the kind of pick itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The dialog's title. `None` leaves the platform's default ("Open",
    /// "Save", the folder dialog's own).
    pub title: Option<String>,
    /// A suggested file name — the save dialog's pre-filled name field, or
    /// the file dialogs' starting file. Ignored by the directory pick.
    pub suggested_name: Option<String>,
}

impl Options {
    /// No title, no suggested name — the platform defaults throughout.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Ask the user to choose an existing file.
///
/// `Ok(None)` is the user cancelling; see the crate doc for why that is not
/// an error. Check [`SUPPORTED`] first if you need to hide the affordance
/// rather than offer one that refuses.
pub fn pick_file(options: &Options) -> Result<Option<PathBuf>, String> {
    pick_file_arm(options)
}

/// Ask the user where to save a file.
///
/// The dialog confirms overwriting; the path is not created, and the file is
/// not written — this crate asks, the caller acts. `Ok(None)` is the user
/// cancelling.
pub fn pick_save_file(options: &Options) -> Result<Option<PathBuf>, String> {
    pick_save_file_arm(options)
}

/// Ask the user to choose an existing directory.
///
/// `Ok(None)` is the user cancelling.
pub fn pick_directory(options: &Options) -> Result<Option<PathBuf>, String> {
    pick_directory_arm(options)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_default_is_the_platforms_own() {
        // "No options" must mean the platform's defaults, not an implicit
        // title or name the caller never chose.
        let o = Options::new();
        assert_eq!(o.title, None);
        assert_eq!(o.suggested_name, None);
    }

    // Like `open-url`, there is deliberately no happy-path test: a successful
    // pick shows a real modal dialog and waits for a human. The verification
    // path is the `floom-capabilities` gallery's action button, which a human
    // presses and answers. What *is* testable is the refusal vocabulary, and
    // only on a target that has one.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    #[test]
    fn the_refusal_names_the_missing_plumbing() {
        let err = pick_file(&Options::new()).unwrap_err();
        // A refusal that said only "not supported" would hide the reason and
        // the way out; the assertion pins both halves of the sentence.
        assert!(err.contains("platform-layer plumbing"), "{err}");
        assert!(err.contains("not available on"), "{err}");
    }

    /// The flag claims a dialog exactly where the crate has one.
    ///
    /// The predicate is written out here on purpose: `SUPPORTED` is generated
    /// from the arm list, and this is the independent restatement that catches
    /// the flag being changed to something else — hard-coded, or reading a
    /// different list. A desktop that gained a fourth arm has to update this
    /// line too, which is the point.
    ///
    /// Deliberately **not** written as "if `SUPPORTED` then call it": on a
    /// target with an arm, that call opens a modal dialog. A drift must cost a
    /// red assertion, never a dialog on someone's screen.
    #[test]
    fn the_flag_claims_a_dialog_exactly_where_the_crate_has_one() {
        let has_an_arm = cfg!(all(
            any(
                target_os = "linux",
                target_os = "macos",
                target_os = "windows"
            ),
            not(target_env = "ohos")
        ));
        assert_eq!(
            SUPPORTED, has_an_arm,
            "SUPPORTED and the arms disagree — one of them is claiming a dialog the other does not have"
        );
    }

    /// And where there is no dialog, every entry point says so.
    ///
    /// This is the only target set where the promise `SUPPORTED == false`
    /// makes is observable without a human answering a dialog, so it is the
    /// only place the promise is checked.
    #[cfg(any(target_os = "android", target_os = "ios"))]
    #[test]
    fn unsupported_means_every_call_refuses() {
        assert!(
            !SUPPORTED,
            "the phone arms have no dialog, so the flag must not claim one"
        );
        let o = Options::new();
        assert!(pick_file(&o).is_err());
        assert!(pick_save_file(&o).is_err());
        assert!(pick_directory(&o).is_err());
    }
}
