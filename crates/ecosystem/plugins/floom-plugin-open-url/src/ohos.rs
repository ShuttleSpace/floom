//! OpenHarmony routes URLs through `startAbility`, which this crate has not
//! wired to yet.

pub fn open_url(_url: &str) -> Result<(), String> {
    Err(
        "open_url: OpenHarmony routing is `startAbility`, which this crate has \
         not wired yet"
            .into(),
    )
}
