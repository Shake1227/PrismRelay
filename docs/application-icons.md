# Local application icons

The scanner can display the Minecraft Launcher and Lunar Client icons already installed on the current computer. The icons remain local display data and are never included in share codes or game configuration backups. Missing, unreadable, damaged, or unsupported images leave the existing interface icon in place.

On macOS, candidates are limited to the named launcher bundles in `/Applications` and the user's `Applications` folder. The installed Minecraft bundle's `Contents/Resources/favicon.icns` and Lunar bundle's `Contents/Resources/icon.icns` were confirmed to contain PNG icon chunks. The app extracts those chunks without copying the original icons into the repository.

On Windows, candidates are limited to launcher install folders and the Minecraft Launcher package's application assets. Lunar's per-user install location follows its [official shortcut instructions](https://support.lunarclient.com/solutions/no-lunar-client-shortcut). The scanner also checks the named Minecraft Launcher installation folders, limited matching WindowsApps packages, and the usual XboxGames launcher content folder. It does not traverse account folders, game settings, user caches, or arbitrary drives. Inaccessible Store installation folders are skipped without changing their permissions.

PNG images and PNG payloads in ICNS, ICO, or an executable's `RT_ICON` resource are supported. The executable reader follows bounded PE resource tables; it neither executes the program nor scans its other content for image signatures. Unsupported bitmap icon formats are skipped.

Every image is decoded with size, dimension, and memory limits, then re-encoded from its pixels as PNG. Ancestor links and symlink files are rejected. Source metadata and unrelated container bytes are discarded. Successful results are supplied as local PNG data URLs, and unchanged application resources are cached to keep subsequent settings scans responsive.
