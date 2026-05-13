use std::path::PathBuf;

/// Resolve `%LOCALAPPDATA%\Programs\Ankama Launcher\Ankama Launcher.exe`.
/// Returns `None` if the env var is missing or the file does not exist.
pub fn default_launcher_path() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    let p = PathBuf::from(local)
        .join("Programs")
        .join("Ankama Launcher")
        .join("Ankama Launcher.exe");
    p.exists().then_some(p)
}

/// Resolve `%LOCALAPPDATA%\Ganymede\ganymede.exe`.
pub fn default_ganymede_path() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    let p = PathBuf::from(local).join("Ganymede").join("ganymede.exe");
    p.exists().then_some(p)
}

/// String form of the path the UI uses as the placeholder for the launcher
/// exe input — even when the file isn't present, we want the user to see
/// where Doclick *would* look.
pub fn default_launcher_path_hint() -> Option<String> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(
        PathBuf::from(local)
            .join("Programs")
            .join("Ankama Launcher")
            .join("Ankama Launcher.exe")
            .to_string_lossy()
            .into_owned(),
    )
}

pub fn default_ganymede_path_hint() -> Option<String> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(
        PathBuf::from(local)
            .join("Ganymede")
            .join("ganymede.exe")
            .to_string_lossy()
            .into_owned(),
    )
}
