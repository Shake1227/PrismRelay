import json
import pathlib
import subprocess

root = pathlib.Path(__file__).resolve().parents[1]
metadata = json.loads(subprocess.check_output(["cargo", "metadata", "--manifest-path", str(root / "src-tauri/Cargo.toml"), "--format-version", "1", "--locked"], text=True))
sections = ["Prism Relay — Third-party license notices\n\nThe application is GPL-3.0-or-later. Dependencies retain their own licenses. This file includes notices from locked Rust and production JavaScript dependencies, including platform-specific dependencies.\n"]

def notices(directory):
    result = []
    for path in sorted(directory.rglob("*")):
        if len(path.relative_to(directory).parts) > 4 or not path.is_file():
            continue
        name = path.name.lower()
        if name.startswith(("license", "licence", "copying", "notice")) and path.stat().st_size < 150000:
            try:
                result.append((str(path.relative_to(directory)), path.read_text()))
            except UnicodeError:
                continue
    return result

def append(name, version, license_name, repository, directory):
    section = f"\n{'=' * 72}\n{name} {version}\nLicense: {license_name or 'See upstream notices'}\nSource: {repository or 'See package registry'}\n"
    for filename, text in notices(directory):
        section += f"\n{filename}\n{'-' * 48}\n{text.rstrip()}\n"
    sections.append(section)

for package in sorted(metadata["packages"], key=lambda item: (item["name"], item["version"])):
    if package["source"]:
        append(package["name"], package["version"], package["license"], package["repository"], pathlib.Path(package["manifest_path"]).parent)

lock = json.loads((root / "package-lock.json").read_text())
for location, package in sorted(lock["packages"].items()):
    if not location or package.get("dev"):
        continue
    directory = root / location
    manifest = json.loads((directory / "package.json").read_text())
    repository = manifest.get("repository", {})
    if isinstance(repository, dict):
        repository = repository.get("url", "")
    append(manifest["name"], manifest["version"], manifest.get("license"), repository, directory)

output = "\n".join(sections)
(root / "THIRD_PARTY_NOTICES.txt").write_text(output)
(root / "public").mkdir(exist_ok=True)
(root / "public/third-party-notices.txt").write_text(output)
print(f"Included {len(sections) - 1} dependency records ({len(output.encode())} bytes)")
