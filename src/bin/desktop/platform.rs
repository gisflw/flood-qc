//! Per-user application launcher and icon identity.
#[cfg(any(target_os = "linux", test))]
use std::path::Path;

pub fn register_desktop_entry() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        let data = std::env::var_os("XDG_DATA_HOME")
            .filter(|path| Path::new(path).is_absolute())
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|root| Path::new(&root).join(".local/share")))
            .ok_or("Cannot locate the user application directory")?;
        let executable = std::env::current_exe().map_err(|error| error.to_string())?;
        install(&data, &executable)
            .map_err(|error| format!("Desktop icon registration: {error}"))?;
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn desktop_entry(executable: &Path) -> String {
    // Desktop Entry quoting has its own escaping, independent of shell quoting.
    let path = executable
        .to_string_lossy()
        .replace('\\', "\\\\\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%");
    super::icons::desktop_entry()
        .replace("TryExec=mgb-ops-desktop\n", "")
        .replace("Exec=mgb-ops-desktop\n", &format!("Exec=\"{path}\"\n"))
        + "X-MGB-Ops-Managed=true\n"
}

#[cfg(any(target_os = "linux", test))]
fn install(data: &Path, executable: &Path) -> std::io::Result<()> {
    let icons = super::icons::installation_icons();
    for (size, bytes) in icons {
        write_changed(
            &data.join(format!(
                "icons/hicolor/{size}x{size}/apps/{}.png",
                super::WINDOW_APP_ID
            )),
            bytes,
        )?;
    }
    let entry = data.join(format!("applications/{}.desktop", super::WINDOW_APP_ID));
    let existing = std::fs::read_to_string(&entry).ok();
    if existing.as_ref().is_none_or(|entry| {
        entry.contains("X-MGB-Ops-Managed=true") || entry == super::icons::desktop_entry()
    }) {
        write_changed(&entry, desktop_entry(executable).as_bytes())?;
    } else if let Some(existing) = existing {
        // Keep a custom launcher command (including wrappers and arguments),
        // but repair this application's icon and window association as well.
        let mut lines = Vec::new();
        let mut main = false;
        let mut icon = false;
        let mut class = false;
        for line in existing.lines() {
            if line.starts_with('[') {
                if main {
                    if !icon {
                        lines.push(format!("Icon={}", super::WINDOW_APP_ID));
                    }
                    if !class {
                        lines.push(format!("StartupWMClass={}", super::WINDOW_APP_ID));
                    }
                }
                main = line == "[Desktop Entry]";
            }
            if main && line.starts_with("Icon=") {
                icon = true;
                lines.push(format!("Icon={}", super::WINDOW_APP_ID));
            } else if main && line.starts_with("StartupWMClass=") {
                class = true;
                lines.push(format!("StartupWMClass={}", super::WINDOW_APP_ID));
            } else {
                lines.push(line.to_owned());
            }
        }
        if main {
            if !icon {
                lines.push(format!("Icon={}", super::WINDOW_APP_ID));
            }
            if !class {
                lines.push(format!("StartupWMClass={}", super::WINDOW_APP_ID));
            }
        }
        write_changed(&entry, (lines.join("\n") + "\n").as_bytes())?;
    }
    Ok(())
}

#[cfg(any(target_os = "linux", test))]
fn write_changed(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    if std::fs::read(path).is_ok_and(|existing| existing == bytes) {
        return Ok(());
    }
    let parent = path.parent().expect("desktop assets have a parent");
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn launcher_resolves_icons_and_updates_a_moved_executable_without_overwriting_custom_entries() {
        std::fs::create_dir_all("runs").unwrap();
        let root = tempfile::tempdir_in("runs").unwrap();
        install(root.path(), Path::new("/opt/MGB Ops/desktop")).unwrap();
        let entry = root.path().join(format!(
            "applications/{}.desktop",
            super::super::WINDOW_APP_ID
        ));
        assert!(
            std::fs::read_to_string(&entry)
                .unwrap()
                .contains("Exec=\"/opt/MGB Ops/desktop\"")
        );
        let icon = root.path().join(format!(
            "icons/hicolor/256x256/apps/{}.png",
            super::super::WINDOW_APP_ID
        ));
        assert!(icon.is_file());
        install(root.path(), Path::new("/opt/new-desktop")).unwrap();
        assert!(
            std::fs::read_to_string(&entry)
                .unwrap()
                .contains("Exec=\"/opt/new-desktop\"")
        );
        std::fs::write(&entry, "[Desktop Entry]\nExec=custom\n").unwrap();
        install(root.path(), Path::new("/opt/desktop")).unwrap();
        let custom = std::fs::read_to_string(entry).unwrap();
        assert!(custom.contains("Exec=custom\n"));
        let icon_key = format!("Icon={}\n", super::super::WINDOW_APP_ID);
        assert!(custom.contains(&icon_key));
    }
}
