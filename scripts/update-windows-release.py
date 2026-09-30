import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import subprocess
import sys
import tomllib
import zipfile

TAG = "v1.0.0"
TAG_COMMIT = "c904cb565554f1458f5e52f9c7ac47766d96f4bf"
WINDOWS_SOURCE = "prism-relay-v1.0.0-windows-source.tar.gz"
ORIGINAL_SOURCE = "prism-relay-v1.0.0-source.tar.gz"
TARGETS = {
    "x86_64-pc-windows-msvc": "windows-x64",
    "aarch64-pc-windows-msvc": "windows-arm64",
}
WINDOWS_FILES = [
    "prism-relay-" + platform + suffix
    for platform in TARGETS.values()
    for suffix in (".msi", "-setup.exe")
]
MAC_FILES = ["prism-relay-macos-apple-silicon.dmg", "prism-relay-macos-intel.dmg"]
MAX_FILE = 512 * 1024 * 1024


class UpdateError(Exception):
    pass


def require(condition, message):
    if not condition:
        raise UpdateError(message)


def command(arguments, stdout=subprocess.PIPE):
    result = subprocess.run(arguments, stdout=stdout, stderr=subprocess.PIPE, timeout=600)
    require(result.returncode == 0, "Command failed: " + " ".join(arguments[:3]))
    return result.stdout


class Github:
    def __init__(self, repository):
        require(bool(re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository)), "Invalid repository")
        self.repository = repository

    def api(self, endpoint):
        return json.loads(command(["gh", "api", "--hostname", "github.com", "repos/" + self.repository + "/" + endpoint]))

    def artifact(self, artifact_id, destination):
        with destination.open("wb") as output:
            command(["gh", "api", "--hostname", "github.com", "repos/" + self.repository + "/actions/artifacts/" + str(artifact_id) + "/zip"], stdout=output)

    def download(self, destination):
        command(["gh", "release", "download", TAG, "--repo", "github.com/" + self.repository, "--dir", str(destination)])

    def upload(self, path):
        command(["gh", "release", "upload", TAG, str(path), "--repo", "github.com/" + self.repository, "--clobber"])

    def delete(self, name):
        command(["gh", "release", "delete-asset", TAG, name, "--repo", "github.com/" + self.repository, "--yes"])

    def edit(self, notes):
        command(["gh", "release", "edit", TAG, "--repo", "github.com/" + self.repository, "--notes-file", str(notes)])


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def read_plan(state):
    return json.loads((state / "plan.json").read_text(encoding="utf-8"))


def validate_run(client, run_id):
    require(bool(re.fullmatch(r"[1-9][0-9]{0,19}", str(run_id))), "CI run ID must be a positive integer")
    run = client.api("actions/runs/" + str(run_id))
    workflow = client.api("actions/workflows/ci.yml")
    require(run.get("id") == int(run_id), "CI run ID mismatch")
    require(run.get("workflow_id") == workflow.get("id") and run.get("path", "").split("@", 1)[0] == ".github/workflows/ci.yml", "The run is not the CI workflow")
    require(run.get("head_branch") == "main" and run.get("event") in {"push", "workflow_dispatch"}, "CI must run on main, without a pull request")
    require(run.get("status") == "completed" and run.get("conclusion") == "success", "CI has not completed successfully")
    for field in ("repository", "head_repository"):
        require(run.get(field, {}).get("full_name", "").lower() == client.repository.lower(), "CI repository mismatch")
    require(bool(re.fullmatch(r"[0-9a-f]{40}", run.get("head_sha", ""))), "Invalid CI source SHA")
    require(isinstance(run.get("run_attempt"), int) and run["run_attempt"] > 0, "Invalid CI run attempt")
    return run


def validate_release(client):
    release = client.api("releases/tags/" + TAG)
    require(release.get("tag_name") == TAG and release.get("draft") is False, "The existing public release is required")
    require(release.get("immutable") is False, "Immutable release assets cannot be replaced")
    reference = client.api("git/ref/tags/" + TAG)["object"]
    for _ in range(4):
        if reference.get("type") != "tag":
            break
        reference = client.api("git/tags/" + reference["sha"])["object"]
    require(reference.get("type") == "commit" and reference.get("sha") == TAG_COMMIT, "The original release tag moved")
    names = set()
    for asset in release.get("assets", []):
        name = asset.get("name", "")
        require(bool(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,199}", name)) and name not in names, "Unsafe or duplicate release asset name")
        require(isinstance(asset.get("size"), int) and 0 < asset["size"] <= MAX_FILE, "Invalid release asset size")
        names.add(name)
    require(set(WINDOWS_FILES + MAC_FILES + [ORIGINAL_SOURCE, "LICENSE", "THIRD_PARTY_NOTICES.txt", "SHA256SUMS.txt"]) <= names, "The release is missing required assets")
    return release


def snapshot(release):
    return {
        "id": release["id"],
        "body": release.get("body") or "",
        "assets": sorted(
            [{key: asset.get(key) for key in ("id", "name", "size", "digest")} for asset in release["assets"]],
            key=lambda asset: asset["name"],
        ),
    }


def inspect(client, state, run_id):
    run = validate_run(client, run_id)
    artifacts = []
    for page in range(1, 11):
        result = client.api("actions/runs/" + str(run_id) + "/artifacts?per_page=100&page=" + str(page))
        artifacts.extend(result["artifacts"])
        if len(result["artifacts"]) < 100:
            break
    else:
        raise UpdateError("Too many CI artifacts")
    selected = {}
    for target in TARGETS:
        matches = [artifact for artifact in artifacts if artifact.get("name") == "windows-installers-" + target]
        require(len(matches) == 1 and matches[0].get("expired") is False, "Missing, expired or duplicate Windows artifact: " + target)
        artifact = matches[0]
        origin = artifact.get("workflow_run", {})
        require(origin.get("id") == int(run_id) and origin.get("head_sha") == run["head_sha"] and origin.get("head_branch") == "main", "Artifact source does not match the selected CI run")
        require(bool(re.fullmatch(r"sha256:[0-9a-f]{64}", artifact.get("digest", ""))), "Artifact digest is missing")
        selected[target] = {key: artifact[key] for key in ("id", "digest", "name")}
    release = validate_release(client)
    state.mkdir(parents=True, exist_ok=False)
    write_json(state / "plan.json", {
        "repository": client.repository, "ciRunId": int(run_id), "ciRunAttempt": run["run_attempt"],
        "sourceCommit": run["head_sha"], "artifacts": selected, "release": snapshot(release),
    })
    output = os.environ.get("GITHUB_OUTPUT")
    if output:
        with open(output, "a", encoding="utf-8") as stream:
            stream.write("source_sha=" + run["head_sha"] + "\n")
    print("Validated successful main CI run and both Windows artifacts")


def check_download(directory, assets):
    require({path.name for path in directory.iterdir()} == {asset["name"] for asset in assets}, "Downloaded release asset set differs")
    for asset in assets:
        path = directory / asset["name"]
        require(path.is_file() and not path.is_symlink() and path.stat().st_size == asset["size"], "Downloaded release asset size or type differs: " + asset["name"])
        if asset.get("digest"):
            require("sha256:" + digest(path) == asset["digest"], "Downloaded release asset digest differs: " + asset["name"])


def check_checksums(directory):
    entries = set()
    for line in (directory / "SHA256SUMS.txt").read_text(encoding="utf-8").splitlines():
        match = re.fullmatch(r"([0-9a-fA-F]{64}) [ *]([A-Za-z0-9][A-Za-z0-9._-]{0,199})", line)
        require(match is not None, "Invalid release checksum entry")
        expected, name = match.groups()
        require(name not in entries and name != "SHA256SUMS.txt" and (directory / name).is_file(), "Duplicate or unknown release checksum entry")
        require(digest(directory / name) == expected.lower(), "Existing release checksum differs: " + name)
        entries.add(name)
    require(set(WINDOWS_FILES + MAC_FILES + [ORIGINAL_SOURCE]) <= entries, "Missing existing installer or source checksums")


def check_source(source, sha):
    require(command(["git", "-C", str(source), "rev-parse", "HEAD"]).decode().strip() == sha, "Checked-out source does not match CI")
    require(command(["git", "-C", str(source), "rev-parse", TAG + "^{commit}"]).decode().strip() == TAG_COMMIT, "Checked-out release tag moved")
    command(["git", "-C", str(source), "merge-base", "--is-ancestor", sha, "origin/main"])
    package = json.loads((source / "package.json").read_text(encoding="utf-8"))
    lock = json.loads((source / "package-lock.json").read_text(encoding="utf-8"))
    tauri = json.loads((source / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    cargo = tomllib.loads((source / "src-tauri/Cargo.toml").read_text(encoding="utf-8"))
    cargo_lock = tomllib.loads((source / "src-tauri/Cargo.lock").read_text(encoding="utf-8"))
    versions = [package["version"], lock["version"], lock["packages"][""]["version"], tauri["version"], cargo["package"]["version"]]
    versions.extend(entry["version"] for entry in cargo_lock["package"] if entry["name"] == "prism-relay")
    require(len(versions) == 6 and all(version == TAG[1:] for version in versions), "Source versions do not match v1.0.0")
    require((source / "docs/release-notes.md").read_text(encoding="utf-8").splitlines()[0] == "# Prism Relay 1.0.0", "Release notes version differs")


def prepare(client, state, source):
    plan = read_plan(state)
    require(plan["repository"] == client.repository, "Update repository changed")
    run = validate_run(client, plan["ciRunId"])
    require(run["head_sha"] == plan["sourceCommit"] and run["run_attempt"] == plan["ciRunAttempt"], "CI changed after validation")
    require(snapshot(validate_release(client)) == plan["release"], "Release changed after validation")
    check_source(source, plan["sourceCommit"])
    backup = state / "backup"
    assets = backup / "assets"
    assets.mkdir(parents=True)
    client.download(assets)
    check_download(assets, plan["release"]["assets"])
    check_checksums(assets)
    (backup / "release-body.md").write_text(plan["release"]["body"], encoding="utf-8")
    write_json(backup / "release.json", plan["release"])
    expected = {path.name: digest(path) for path in assets.iterdir()}
    write_json(backup / "SHA256.json", expected)
    output = state / "output"
    output.mkdir()
    for target, platform in TARGETS.items():
        artifact = plan["artifacts"][target]
        archive_path = state / (target + ".zip")
        client.artifact(artifact["id"], archive_path)
        require(archive_path.stat().st_size <= MAX_FILE and "sha256:" + digest(archive_path) == artifact["digest"], "CI artifact digest differs")
        names = {"prism-relay-" + platform + suffix for suffix in (".msi", "-setup.exe")}
        with zipfile.ZipFile(archive_path) as archive:
            require(len(archive.infolist()) == 2 and {entry.filename for entry in archive.infolist()} == names, "Unexpected CI artifact files")
            for entry in archive.infolist():
                require(not entry.is_dir() and (entry.external_attr >> 16) & 0o170000 != 0o120000 and 0 < entry.file_size <= MAX_FILE, "Invalid CI artifact member")
                path = output / entry.filename
                with archive.open(entry) as stream, path.open("wb") as destination:
                    shutil.copyfileobj(stream, destination)
                require(path.stat().st_size == entry.file_size, "Extracted installer size differs")
                with path.open("rb") as stream:
                    magic = stream.read(8)
                require(magic.startswith(b"MZ") if path.suffix == ".exe" else magic == bytes.fromhex("d0cf11e0a1b11ae1"), "Installer file type differs")
    for name in ("LICENSE", "THIRD_PARTY_NOTICES.txt"):
        require((source / name).read_bytes() == (assets / name).read_bytes(), "License notices changed; a broader release update is required")
    command(["git", "-C", str(source), "-c", "tar.umask=0002", "archive", "--format=tar.gz", "--prefix=prism-relay-v1.0.0-windows/", plan["sourceCommit"], "-o", str((output / WINDOWS_SOURCE).resolve())])
    (state / "release-notes.md").write_bytes((source / "docs/release-notes.md").read_bytes())
    info = {
        "releaseTag": TAG, "tagCommit": TAG_COMMIT,
        "windows": {"sourceCommit": plan["sourceCommit"], "sourceArchive": WINDOWS_SOURCE, "ciRunId": plan["ciRunId"], "ciRunAttempt": plan["ciRunAttempt"], "artifacts": plan["artifacts"]},
        "macOS": {"sourceCommit": TAG_COMMIT, "sourceArchive": ORIGINAL_SOURCE, "assets": {name: expected[name] for name in MAC_FILES}},
    }
    write_json(output / "BUILD_INFO.json", info)
    plan["expectedAssets"] = expected
    plan["newAssets"] = [WINDOWS_SOURCE, "BUILD_INFO.json", *WINDOWS_FILES, "SHA256SUMS.txt"]
    plan["preparedAssets"] = {name: digest(output / name) for name in WINDOWS_FILES + [WINDOWS_SOURCE, "BUILD_INFO.json"]}
    write_json(state / "plan.json", plan)
    print("Prepared four Windows installers and corresponding source; all original assets are backed up")


def checksum_output(state, plan, backup_artifact_id):
    output = state / "output"
    info = json.loads((output / "BUILD_INFO.json").read_text(encoding="utf-8"))
    info["backupArtifactId"] = int(backup_artifact_id)
    info["windows"]["sha256"] = {name: digest(output / name) for name in WINDOWS_FILES + [WINDOWS_SOURCE]}
    write_json(output / "BUILD_INFO.json", info)
    expected = dict(plan["expectedAssets"])
    expected.update({name: digest(output / name) for name in plan["newAssets"] if name != "SHA256SUMS.txt"})
    (output / "SHA256SUMS.txt").write_text("".join(expected[name] + "  " + name + "\n" for name in sorted(expected) if name != "SHA256SUMS.txt"), encoding="utf-8")
    expected["SHA256SUMS.txt"] = digest(output / "SHA256SUMS.txt")
    return expected


def rollback(client, state, plan, attempted):
    errors = []
    try:
        current = {asset["name"] for asset in client.api("releases/tags/" + TAG)["assets"]}
    except Exception:
        current = set(attempted)
    for name in reversed(attempted):
        try:
            if name in plan["expectedAssets"]:
                client.upload(state / "backup/assets" / name)
            elif name in current:
                client.delete(name)
        except Exception:
            errors.append(name)
    try:
        client.edit(state / "backup/release-body.md")
    except Exception:
        errors.append("release body")
    if not errors:
        try:
            release = validate_release(client)
            require({asset["name"] for asset in release["assets"]} == set(plan["expectedAssets"]), "Recovered asset set differs")
            require((release.get("body") or "") == plan["release"]["body"], "Recovered release body differs")
            recovered = state / "recovered"
            recovered.mkdir()
            client.download(recovered)
            check_download(recovered, release["assets"])
            require(all(digest(recovered / name) == checksum for name, checksum in plan["expectedAssets"].items()), "Recovered release bytes differ")
        except Exception:
            errors.append("restored asset verification")
    require(not errors, "Recovery incomplete; restore the saved Actions backup: " + ", ".join(errors))


def publish(client, state, backup_artifact_id):
    require(bool(re.fullmatch(r"[1-9][0-9]{0,19}", str(backup_artifact_id))), "A saved Actions backup artifact is required")
    plan = read_plan(state)
    require(plan["repository"] == client.repository, "Update repository changed")
    require(snapshot(validate_release(client)) == plan["release"], "Release changed before publication")
    run = validate_run(client, plan["ciRunId"])
    require(run["head_sha"] == plan["sourceCommit"] and run["run_attempt"] == plan["ciRunAttempt"], "CI changed before publication")
    for name, expected in plan["expectedAssets"].items():
        require(digest(state / "backup/assets" / name) == expected, "The original release backup changed")
    for name, expected in plan["preparedAssets"].items():
        require(digest(state / "output" / name) == expected, "Prepared release output changed")
    expected = checksum_output(state, plan, backup_artifact_id)
    attempted = []
    try:
        for name in plan["newAssets"]:
            attempted.append(name)
            client.upload(state / "output" / name)
        client.edit(state / "release-notes.md")
        release = validate_release(client)
        require({asset["name"] for asset in release["assets"]} == set(expected), "Published release asset set differs")
        verified = state / "verified"
        verified.mkdir()
        client.download(verified)
        check_download(verified, release["assets"])
        require(all(digest(verified / name) == checksum for name, checksum in expected.items()), "Published release bytes differ")
        check_checksums(verified)
    except Exception as failure:
        rollback(client, state, plan, attempted)
        raise UpdateError("Windows update failed; original assets and release body were restored") from failure
    print("Updated Windows assets and checksums; the original tag, macOS installers and source are unchanged")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("phase", choices=("inspect", "prepare", "publish"))
    parser.add_argument("--state", type=pathlib.Path, required=True)
    parser.add_argument("--ci-run-id")
    parser.add_argument("--source", type=pathlib.Path)
    parser.add_argument("--backup-artifact-id")
    arguments = parser.parse_args()
    client = Github(os.environ.get("GITHUB_REPOSITORY", ""))
    if arguments.phase == "inspect":
        inspect(client, arguments.state, arguments.ci_run_id)
    elif arguments.phase == "prepare":
        require(arguments.source is not None, "The checked-out CI source is required")
        prepare(client, arguments.state, arguments.source)
    else:
        publish(client, arguments.state, arguments.backup_artifact_id)


if __name__ == "__main__":
    try:
        main()
    except (UpdateError, OSError, ValueError, KeyError, zipfile.BadZipFile, subprocess.SubprocessError) as error:
        print("Windows release update failed: " + str(error), file=sys.stderr)
        sys.exit(1)
