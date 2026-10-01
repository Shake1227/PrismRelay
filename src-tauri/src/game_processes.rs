use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Game {
    Minecraft,
    Lunar,
}

impl Game {
    fn label(self) -> &'static str {
        match self {
            Self::Minecraft => "Minecraft",
            Self::Lunar => "Lunar Client",
        }
    }
}

pub fn detect() -> Vec<String> {
    let system = sysinfo::System::new_all();
    let mut found = BTreeSet::new();
    for (pid, process) in system.processes() {
        if pid.as_u32() == std::process::id() {
            continue;
        }
        if let Some(game) = classify(process.name(), process.exe(), process.cmd()) {
            found.insert(game.label().to_owned());
        }
    }
    found.into_iter().collect()
}

fn is_java(name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| {
        let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
        ["java", "javaw", "java.exe", "javaw.exe"]
            .iter()
            .any(|candidate| name.eq_ignore_ascii_case(candidate))
    })
}

fn classify(name: &OsStr, executable: Option<&Path>, command: &[OsString]) -> Option<Game> {
    if command.len() > 4096
        || (!is_java(name) && !executable.is_some_and(|path| is_java(path.as_os_str())))
    {
        return None;
    }
    let (main, arguments) = java_main(command)?;
    match main {
        "net.minecraft.client.main.Main"
        | "net.minecraft.client.Minecraft"
        | "net.fabricmc.loader.impl.launch.knot.KnotClient"
        | "net.fabricmc.loader.launch.knot.KnotClient"
        | "org.quiltmc.loader.impl.launch.knot.KnotClient" => Some(Game::Minecraft),
        "com.moonsworth.lunar.genesis.Genesis"
            if has_value(arguments, "--version")
                && has_value(arguments, "--classpathDir")
                && has_value(arguments, "--workingDirectory") =>
        {
            Some(Game::Lunar)
        }
        "net.minecraft.launchwrapper.Launch"
            if has_value(arguments, "--gameDir")
                && (has_value(arguments, "--version")
                    || arguments.windows(2).any(|pair| {
                        pair[0] == "--tweakClass"
                            && pair[1]
                                .to_str()
                                .is_some_and(|class| class.ends_with(".FMLTweaker"))
                    })) =>
        {
            Some(Game::Minecraft)
        }
        "cpw.mods.bootstraplauncher.BootstrapLauncher" | "cpw.mods.modlauncher.Launcher"
            if arguments.windows(2).any(|pair| {
                pair[0] == "--launchTarget"
                    && matches!(
                        pair[1].to_str(),
                        Some("forgeclient" | "neoforgeclient" | "fmlclient")
                    )
            }) =>
        {
            Some(Game::Minecraft)
        }
        _ => None,
    }
}

fn has_value(arguments: &[OsString], flag: &str) -> bool {
    arguments.iter().enumerate().any(|(index, argument)| {
        let Some(argument) = argument.to_str() else {
            return false;
        };
        let value = if argument == flag {
            arguments.get(index + 1).and_then(|value| value.to_str())
        } else {
            argument
                .strip_prefix(flag)
                .and_then(|suffix| suffix.strip_prefix('='))
        };
        value.is_some_and(|value| !value.is_empty() && !value.starts_with('-'))
    })
}

fn java_main(command: &[OsString]) -> Option<(&str, &[OsString])> {
    let mut index = usize::from(command.first().is_some_and(|arg| is_java(arg)));
    while let Some(argument) = command.get(index)?.to_str() {
        match argument {
            "-jar" => {
                let jar = command.get(index + 1)?.to_str()?;
                if jar.rsplit(['/', '\\']).next() != Some("genesis-0.1.0-SNAPSHOT-all.jar") {
                    return None;
                }
                return Some((
                    "com.moonsworth.lunar.genesis.Genesis",
                    &command[index + 2..],
                ));
            }
            "-m" | "--module" => return None,
            "-cp"
            | "-classpath"
            | "--class-path"
            | "-p"
            | "--module-path"
            | "--upgrade-module-path"
            | "--add-modules"
            | "--add-exports"
            | "--add-opens"
            | "--add-reads"
            | "--patch-module"
            | "--enable-native-access"
            | "--source" => {
                command.get(index + 1)?;
                index += 2;
            }
            "--" => {
                index += 1;
                return Some((command.get(index)?.to_str()?, &command[index + 1..]));
            }
            _ if argument.starts_with('@') => return None,
            _ if argument.starts_with('-') => index += 1,
            _ => return Some((argument, &command[index + 1..])),
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify_arguments(name: &str, executable: &str, arguments: &[&str]) -> Option<Game> {
        let command: Vec<_> = arguments.iter().map(OsString::from).collect();
        classify(OsStr::new(name), Some(Path::new(executable)), &command)
    }

    #[test]
    fn launchers_helpers_self_and_unrelated_java_are_not_games() {
        for name in [
            "Lunar Client",
            "Lunar Client Helper",
            "Lunar Client Helper (Renderer)",
            "Minecraft Launcher",
            "MinecraftLauncher.exe",
            "Lunar Client.exe",
            "Prism Relay",
            "prism-relay.exe",
        ] {
            assert_eq!(
                classify_arguments(name, name, &[name, "net.minecraft.client.main.Main"]),
                None
            );
        }
        for arguments in [
            vec!["java", "-jar", "unrelated.jar"],
            vec!["java", "org.example.Main", "net.minecraft.client.main.Main"],
            vec![
                "java",
                "-cp",
                "net.minecraft.client.main.Main",
                "org.example.Main",
            ],
            vec![
                "java",
                "-Dmessage=net.minecraft.client.main.Main",
                "org.example.Main",
            ],
            vec!["java", "com.moonsworth.lunar.genesis.Genesis"],
        ] {
            assert_eq!(
                classify_arguments("java", "/runtime/bin/java", &arguments),
                None
            );
        }
    }

    #[test]
    fn macos_and_windows_java_game_entrypoints_are_detected() {
        for (name, executable) in [
            ("java", "/runtime/bin/java"),
            ("javaw.exe", r"C:\Runtime\bin\javaw.exe"),
        ] {
            assert_eq!(
                classify_arguments(
                    name,
                    executable,
                    &[
                        executable,
                        "-Xmx2G",
                        "--add-opens",
                        "java.base/java.lang=ALL-UNNAMED",
                        "-cp",
                        "libraries",
                        "net.minecraft.client.main.Main",
                        "--gameDir",
                        "game",
                    ],
                ),
                Some(Game::Minecraft)
            );
            assert_eq!(
                classify_arguments(
                    name,
                    executable,
                    &[
                        executable,
                        "-cp",
                        "genesis.jar",
                        "com.moonsworth.lunar.genesis.Genesis",
                        "--gameDir",
                        "game",
                        "--version",
                        "1.21.11",
                        "--classpathDir",
                        "libraries",
                        "--workingDirectory",
                        "game",
                    ],
                ),
                Some(Game::Lunar)
            );
        }
    }

    #[test]
    fn client_mod_loaders_are_distinct_from_servers() {
        for main in [
            "net.fabricmc.loader.impl.launch.knot.KnotClient",
            "net.fabricmc.loader.launch.knot.KnotClient",
            "org.quiltmc.loader.impl.launch.knot.KnotClient",
        ] {
            assert_eq!(
                classify_arguments("java", "java", &["java", main]),
                Some(Game::Minecraft)
            );
        }
        for target in ["forgeclient", "neoforgeclient", "fmlclient"] {
            assert_eq!(
                classify_arguments(
                    "java",
                    "java",
                    &[
                        "java",
                        "cpw.mods.bootstraplauncher.BootstrapLauncher",
                        "--launchTarget",
                        target
                    ],
                ),
                Some(Game::Minecraft)
            );
        }
        for arguments in [
            vec!["java", "net.fabricmc.loader.impl.launch.knot.KnotServer"],
            vec!["java", "net.minecraft.server.Main"],
            vec![
                "java",
                "cpw.mods.bootstraplauncher.BootstrapLauncher",
                "--launchTarget",
                "forgeserver",
            ],
            vec![
                "java",
                "net.minecraft.launchwrapper.Launch",
                "--tweakClass",
                "net.minecraftforge.fml.common.launcher.FMLServerTweaker",
            ],
        ] {
            assert_eq!(classify_arguments("java", "java", &arguments), None);
        }
    }

    #[test]
    fn only_the_verified_lunar_bootstrap_jar_with_game_arguments_is_detected() {
        for jar in [
            "/client/offline/multiver/genesis-0.1.0-SNAPSHOT-all.jar",
            r"C:\Client\offline\multiver\genesis-0.1.0-SNAPSHOT-all.jar",
        ] {
            assert_eq!(
                classify_arguments(
                    "JAVA.EXE",
                    "JAVA.EXE",
                    &[
                        "JAVA.EXE",
                        "-jar",
                        jar,
                        "--version",
                        "1.21.11",
                        "--classpathDir",
                        "libraries",
                        "--workingDirectory",
                        "game",
                    ],
                ),
                Some(Game::Lunar)
            );
            assert_eq!(
                classify_arguments("java", "java", &["java", "-jar", jar]),
                None
            );
        }
    }

    #[test]
    fn lunar_bootstrap_supports_explicit_key_value_arguments() {
        assert_eq!(
            classify_arguments(
                "java",
                "java",
                &[
                    "java",
                    "com.moonsworth.lunar.genesis.Genesis",
                    "--version=1.21.11",
                    "--classpathDir=libraries",
                    "--workingDirectory=game",
                ],
            ),
            Some(Game::Lunar)
        );
        assert_eq!(
            classify_arguments(
                "java",
                "java",
                &[
                    "java",
                    "com.moonsworth.lunar.genesis.Genesis",
                    "--version=1.21.11",
                    "--classpathDir=",
                    "--workingDirectory=game",
                ],
            ),
            None
        );
    }

    #[test]
    fn argument_files_and_incomplete_commands_are_not_read_or_guessed() {
        for arguments in [
            vec!["java", "@args.txt", "net.minecraft.client.main.Main"],
            vec!["java", "-cp"],
            vec![
                "java",
                "-jar",
                "genesis.jar",
                "--gameDir",
                "game",
                "--version",
                "1.21.11",
            ],
            vec![
                "java",
                "com.moonsworth.lunar.genesis.Genesis",
                "--gameDir",
                "--version",
                "1.21.11",
            ],
        ] {
            assert_eq!(classify_arguments("java", "java", &arguments), None);
        }
    }
}
