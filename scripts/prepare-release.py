import argparse
import hashlib
import json
import os
import pathlib
import re
import shutil
import stat
import subprocess
import sys
import tempfile
import tomllib
import zipfile

PLATFORMS = {
    "windows-x64": (".msi", "-setup.exe"),
    "windows-arm64": (".msi", "-setup.exe"),
    "macos-apple-silicon": (".dmg",),
    "macos-intel": (".dmg",),
}
MAX_FILE = 512 * 1024 * 1024


class ReleaseError(Exception):
    pass


def require(condition, message):
    if not condition:
        raise ReleaseError(message)


def command(arguments, cwd=None, stdout=subprocess.PIPE):
    result = subprocess.run(arguments, cwd=cwd, stdout=stdout, stderr=subprocess.PIPE, timeout=600)
    require(result.returncode == 0, "Command failed: " + " ".join(arguments[:3]))
    return result.stdout


class Github:
    def __init__(self, repository):
        self.repository = repository

    def api(self, endpoint):
        return json.loads(command(["gh", "api", "--hostname", "github.com", "repos/" + self.repository + "/" + endpoint]))

    def artifact(self, artifact_id, destination):
        with destination.open("wb") as stream:
            command(["gh", "api", "--hostname", "github.com", "repos/" + self.repository + "/actions/artifacts/" + str(artifact_id) + "/zip"], stdout=stream)


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def installer_names(platform):
    return {"prism-relay-" + platform + suffix for suffix in PLATFORMS[platform]}


def context(environment):
    tag = environment.get("GITHUB_REF_NAME", "")
    require(environment.get("GITHUB_REF_TYPE") == "tag" and bool(re.fullmatch(r"v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)", tag)), "A stable vX.Y.Z tag is required")
    repository = environment.get("GITHUB_REPOSITORY", "")
    require(bool(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]{0,38}/[A-Za-z0-9_.-]{1,100}", repository)) and repository.split("/", 1)[1] not in {".", ".."}, "Invalid repository")
    identifiers = [environment.get(key, "") for key in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")]
    require(all(re.fullmatch(r"[1-9][0-9]{0,19}", value) for value in identifiers), "Valid run ID and attempt are required")
    return tag, repository, int(identifiers[0]), int(identifiers[1])


def validate_source(source, tag, environment):
    head = command(["git", "rev-parse", "--verify", "HEAD"], cwd=source).decode().strip()
    tagged = command(["git", "rev-parse", "--verify", "refs/tags/" + tag + "^{commit}"], cwd=source).decode().strip()
    require(bool(re.fullmatch(r"[0-9a-f]{40}", head)) and head == tagged, "HEAD does not match the release tag")
    require(environment.get("GITHUB_SHA") == head, "Workflow source SHA does not match HEAD")
    package = json.loads((source / "package.json").read_text(encoding="utf-8"))
    lock = json.loads((source / "package-lock.json").read_text(encoding="utf-8"))
    tauri = json.loads((source / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    cargo = tomllib.loads((source / "src-tauri/Cargo.toml").read_text(encoding="utf-8"))
    cargo_lock = tomllib.loads((source / "src-tauri/Cargo.lock").read_text(encoding="utf-8"))
    versions = [package["version"], lock["version"], lock["packages"][""]["version"], tauri["version"], cargo["package"]["version"]]
    versions.extend(entry["version"] for entry in cargo_lock["package"] if entry["name"] == "prism-relay")
    require(len(versions) == 6 and all(value == tag[1:] for value in versions), "Source versions do not match the release tag")
    require((source / "docs/release-notes.md").read_text(encoding="utf-8").splitlines()[0] == "# Prism Relay " + tag[1:], "Release notes version does not match the tag")
    require(not command(["git", "status", "--porcelain", "--untracked-files=no"], cwd=source).strip(), "Tracked source files were modified")
    return head


def validate_artifacts(client, run_id, run_attempt, head):
    run = client.api("actions/runs/" + str(run_id))
    require(run.get("id") == run_id and run.get("run_attempt") == run_attempt and run.get("head_sha") == head, "Release workflow source or attempt differs")
    require(run.get("path", "").split("@", 1)[0] == ".github/workflows/release.yml" and run.get("event") in {"push", "workflow_dispatch"}, "The run is not the release workflow")
    require(run.get("status") == "in_progress" and run.get("conclusion") is None, "Release workflow is no longer running")
    for field in ("repository", "head_repository"):
        require(run.get(field, {}).get("full_name", "").lower() == client.repository.lower(), "Release workflow repository differs")
    result = client.api("actions/runs/" + str(run_id) + "/artifacts?per_page=100")
    artifacts = result.get("artifacts", [])
    require(result.get("total_count") == 4 and len(artifacts) == 4 and {artifact.get("name") for artifact in artifacts} == set(PLATFORMS), "Exactly four platform artifacts are required")
    selected = {}
    identifiers = set()
    for artifact in artifacts:
        identifier = artifact.get("id")
        require(type(identifier) is int and identifier > 0 and identifier not in identifiers, "Invalid or duplicate artifact ID")
        identifiers.add(identifier)
        require(artifact.get("expired") is False and bool(re.fullmatch(r"sha256:[0-9a-f]{64}", artifact.get("digest", ""))), "Artifact is expired or its digest is missing")
        origin = artifact.get("workflow_run", {})
        require(origin.get("id") == run_id and origin.get("head_sha") == head, "Artifact source does not match the release tag")
        selected[artifact["name"]] = {key: artifact[key] for key in ("id", "name", "digest")}
    return selected


def verify_archive(client, artifact, platform, installers, temporary):
    path = temporary / (platform + ".zip")
    client.artifact(artifact["id"], path)
    require(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= 2 * MAX_FILE, "Invalid artifact archive")
    require("sha256:" + digest(path) == artifact["digest"], "Artifact archive digest differs: " + platform)
    with zipfile.ZipFile(path) as archive:
        entries = archive.infolist()
        names = installer_names(platform)
        require(len(entries) == len(names) and {entry.filename for entry in entries} == names, "Artifact installer names differ: " + platform)
        for entry in entries:
            kind = stat.S_IFMT(entry.external_attr >> 16)
            local = installers / entry.filename
            require(not entry.is_dir() and kind in (0, stat.S_IFREG) and entry.file_size == local.stat().st_size, "Unsafe or mismatched artifact installer")
            with archive.open(entry) as stream:
                expected = hashlib.file_digest(stream, "sha256").hexdigest()
            require(expected == digest(local), "Downloaded installer differs from its verified artifact: " + entry.filename)


def prepare(source, installers, environment, client=None):
    tag, repository, run_id, run_attempt = context(environment)
    require(installers.is_dir() and not installers.is_symlink(), "Invalid installer directory")
    names = set().union(*(installer_names(platform) for platform in PLATFORMS))
    require({path.name for path in installers.iterdir()} == names, "Exactly six expected installers are required")
    for name in names:
        path = installers / name
        require(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= MAX_FILE, "Invalid installer file: " + name)
    head = validate_source(source, tag, environment)
    client = client or Github(repository)
    require(client.repository.lower() == repository.lower(), "GitHub client repository differs")
    artifacts = validate_artifacts(client, run_id, run_attempt, head)
    source_name = "prism-relay-" + tag + "-source.tar.gz"
    with tempfile.TemporaryDirectory(prefix="prism-release-") as folder:
        temporary = pathlib.Path(folder)
        for platform, artifact in artifacts.items():
            verify_archive(client, artifact, platform, installers, temporary)
        command(["git", "-c", "tar.umask=0002", "archive", "--format=tar.gz", "--prefix=prism-relay-" + tag + "/", head, "-o", str(temporary / source_name)], cwd=source)
        for name in ("LICENSE", "THIRD_PARTY_NOTICES.txt"):
            path = source / name
            require(path.is_file() and not path.is_symlink() and 0 < path.stat().st_size <= MAX_FILE, "Invalid source license file")
            shutil.copyfile(path, temporary / name)
        hashes = {name: digest(installers / name) for name in sorted(names)}
        hashes.update({name: digest(temporary / name) for name in (source_name, "LICENSE", "THIRD_PARTY_NOTICES.txt")})
        info = {
            "releaseTag": tag, "tagCommit": head, "sourceCommit": head,
            "sourceArchive": source_name, "repository": repository,
            "buildRunId": run_id, "buildRunAttempt": run_attempt,
            "artifacts": artifacts, "sha256": hashes,
        }
        manifest = temporary / "BUILD_INFO.json"
        manifest.write_text(json.dumps(info, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        sums = dict(hashes)
        sums[manifest.name] = digest(manifest)
        (temporary / "SHA256SUMS.txt").write_text("".join(value + "  " + name + "\n" for name, value in sorted(sums.items())), encoding="utf-8")
        for name in (source_name, "LICENSE", "THIRD_PARTY_NOTICES.txt", "BUILD_INFO.json", "SHA256SUMS.txt"):
            shutil.copyfile(temporary / name, installers / name)
    require({path.name for path in installers.iterdir()} == names | {source_name, "LICENSE", "THIRD_PARTY_NOTICES.txt", "BUILD_INFO.json", "SHA256SUMS.txt"}, "Prepared release asset set differs")
    print("Prepared " + tag + " from " + head + " with six verified installers and complete checksums")
    return info


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--source", type=pathlib.Path, default=pathlib.Path("."))
    parser.add_argument("--artifacts", type=pathlib.Path, required=True)
    arguments = parser.parse_args()
    try:
        prepare(arguments.source.resolve(), arguments.artifacts, os.environ)
    except (ReleaseError, OSError, ValueError, KeyError, TypeError, zipfile.BadZipFile, subprocess.TimeoutExpired) as error:
        print("Release preparation failed: " + str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
