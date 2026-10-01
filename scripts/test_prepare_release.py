import copy
import hashlib
import importlib.util
import io
import json
import pathlib
import subprocess
import tarfile
import tempfile
import unittest
import zipfile

SPEC = importlib.util.spec_from_file_location("release_preparer", pathlib.Path(__file__).with_name("prepare-release.py"))
RELEASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RELEASE)


class FakeGithub:
    def __init__(self, sha, installers):
        self.repository = "Example/PrismRelay"
        self.run = {
            "id": 42, "run_attempt": 1, "head_sha": sha,
            "path": ".github/workflows/release.yml", "event": "push",
            "status": "in_progress", "conclusion": None,
            "repository": {"full_name": self.repository},
            "head_repository": {"full_name": self.repository},
        }
        self.artifacts = []
        self.archives = {}
        self.downloads = []
        for identifier, platform in enumerate(RELEASE.PLATFORMS, 1):
            self.archives[identifier] = self.archive({name: (installers / name).read_bytes() for name in RELEASE.installer_names(platform)})
            self.artifacts.append({
                "id": identifier, "name": platform, "expired": False,
                "digest": "sha256:" + hashlib.sha256(self.archives[identifier]).hexdigest(),
                "workflow_run": {"id": 42, "head_sha": sha},
            })

    @staticmethod
    def archive(files):
        buffer = io.BytesIO()
        with zipfile.ZipFile(buffer, "w") as archive:
            for name, content in files.items():
                archive.writestr(name, content)
        return buffer.getvalue()

    def api(self, endpoint):
        if endpoint == "actions/runs/42":
            return copy.deepcopy(self.run)
        if endpoint == "actions/runs/42/artifacts?per_page=100":
            return {"total_count": len(self.artifacts), "artifacts": copy.deepcopy(self.artifacts)}
        raise AssertionError(endpoint)

    def artifact(self, identifier, destination):
        self.downloads.append(identifier)
        destination.write_bytes(self.archives[identifier])


class PrepareReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="prism-prepare-test-")
        self.root = pathlib.Path(self.temporary.name)
        self.source = self.root / "source"
        self.source.mkdir()
        files = {
            "package.json": '{"version":"1.0.1"}',
            "package-lock.json": '{"version":"1.0.1","packages":{"":{"version":"1.0.1"}}}',
            "src-tauri/tauri.conf.json": '{"version":"1.0.1"}',
            "src-tauri/Cargo.toml": '[package]\nname="prism-relay"\nversion="1.0.1"\n',
            "src-tauri/Cargo.lock": '[[package]]\nname="prism-relay"\nversion="1.0.1"\n',
            "docs/release-notes.md": '# Prism Relay 1.0.1\nSynthetic release notes\n',
            "LICENSE": 'Synthetic GPL fixture\n',
            "THIRD_PARTY_NOTICES.txt": 'Synthetic dependency fixture\n',
            "fixture.txt": 'Synthetic application source\n',
        }
        for name, content in files.items():
            path = self.source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8")
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Synthetic Test")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "core.autocrlf", "false")
        self.git("add", ".")
        self.git("commit", "-m", "Synthetic release source")
        self.git("tag", "-a", "v1.0.1", "-m", "Synthetic version")
        self.sha = self.git("rev-parse", "HEAD").decode().strip()
        self.environment = {
            "GITHUB_REF_TYPE": "tag", "GITHUB_REF_NAME": "v1.0.1",
            "GITHUB_REPOSITORY": "Example/PrismRelay", "GITHUB_SHA": self.sha,
            "GITHUB_RUN_ID": "42", "GITHUB_RUN_ATTEMPT": "1",
        }
        self.installers = self.root / "artifacts"
        self.installers.mkdir()
        self.names = set().union(*(RELEASE.installer_names(platform) for platform in RELEASE.PLATFORMS))
        for name in self.names:
            (self.installers / name).write_bytes(("Synthetic installer: " + name).encode())
        self.client = FakeGithub(self.sha, self.installers)

    def tearDown(self):
        self.temporary.cleanup()

    def git(self, *arguments):
        return subprocess.check_output(["git", "-C", str(self.source), *arguments], stderr=subprocess.PIPE)

    def prepare(self):
        return RELEASE.prepare(self.source, self.installers, self.environment, self.client)

    def assert_no_generated_assets(self):
        self.assertEqual({path.name for path in self.installers.iterdir()}, self.names)

    def test_complete_release_has_matching_source_provenance_and_every_checksum(self):
        info = self.prepare()
        source_name = "prism-relay-v1.0.1-source.tar.gz"
        names = self.names | {source_name, "LICENSE", "THIRD_PARTY_NOTICES.txt", "BUILD_INFO.json", "SHA256SUMS.txt"}
        self.assertEqual({path.name for path in self.installers.iterdir()}, names)
        self.assertEqual(len(names), 11)
        self.assertEqual(info["releaseTag"], "v1.0.1")
        self.assertEqual(info["tagCommit"], self.sha)
        self.assertEqual(info["sourceCommit"], self.sha)
        self.assertEqual(info["sourceArchive"], source_name)
        self.assertEqual((info["buildRunId"], info["buildRunAttempt"]), (42, 1))
        self.assertEqual(info["artifacts"], {artifact["name"]: {key: artifact[key] for key in ("id", "name", "digest")} for artifact in self.client.artifacts})
        self.assertEqual(set(info["sha256"]), names - {"BUILD_INFO.json", "SHA256SUMS.txt"})
        entries = {}
        for line in (self.installers / "SHA256SUMS.txt").read_text().splitlines():
            expected, name = line.split("  ", 1)
            self.assertNotIn(name, entries)
            entries[name] = expected
            self.assertEqual(expected, RELEASE.digest(self.installers / name))
        self.assertEqual(set(entries), names - {"SHA256SUMS.txt"})
        self.assertEqual(len(entries), 10)
        for name, expected in info["sha256"].items():
            self.assertEqual(expected, RELEASE.digest(self.installers / name))
        with tarfile.open(self.installers / source_name, "r:gz") as archive:
            members = {member.name: member for member in archive.getmembers() if member.isfile()}
            tracked = self.git("ls-tree", "-r", "--name-only", self.sha).decode().splitlines()
            self.assertEqual(set(members), {"prism-relay-v1.0.1/" + name for name in tracked})
            for name in tracked:
                member = members["prism-relay-v1.0.1/" + name]
                self.assertEqual(archive.extractfile(member).read(), self.git("show", self.sha + ":" + name))
                self.assertEqual(member.mode, 0o664)
        self.assertEqual(set(self.client.downloads), {1, 2, 3, 4})

    def test_invalid_environment_and_moved_or_modified_source_abort_before_generation(self):
        for key, value in [("GITHUB_REF_TYPE", "branch"), ("GITHUB_REF_NAME", "v1.0.1-rc.1"), ("GITHUB_REF_NAME", "v01.0.1"), ("GITHUB_RUN_ID", "0"), ("GITHUB_RUN_ATTEMPT", ""), ("GITHUB_REPOSITORY", "../outside"), ("GITHUB_SHA", "a" * 40)]:
            with self.subTest(key=key, value=value):
                original = self.environment[key]
                self.environment[key] = value
                with self.assertRaises(RELEASE.ReleaseError):
                    self.prepare()
                self.environment[key] = original
                self.assert_no_generated_assets()
        (self.source / "package.json").write_text('{"version":"1.0.0"}')
        with self.assertRaisesRegex(RELEASE.ReleaseError, "Source versions"):
            self.prepare()
        (self.source / "package.json").write_text('{"version":"1.0.1"}')
        (self.source / "LICENSE").write_text("Modified synthetic license\n")
        with self.assertRaisesRegex(RELEASE.ReleaseError, "modified"):
            self.prepare()
        self.git("add", ".")
        self.git("commit", "-m", "Synthetic newer source")
        with self.assertRaisesRegex(RELEASE.ReleaseError, "HEAD does not match"):
            self.prepare()
        self.assert_no_generated_assets()
        self.assertEqual(self.client.downloads, [])

    def test_missing_extra_empty_or_linked_installers_are_rejected(self):
        path = self.installers / sorted(self.names)[0]
        original = path.read_bytes()
        path.unlink()
        with self.assertRaisesRegex(RELEASE.ReleaseError, "six expected"):
            self.prepare()
        path.write_bytes(original)
        extra = self.installers / "unexpected.txt"
        extra.write_text("Synthetic extra file")
        with self.assertRaisesRegex(RELEASE.ReleaseError, "six expected"):
            self.prepare()
        extra.unlink()
        path.write_bytes(b"")
        with self.assertRaisesRegex(RELEASE.ReleaseError, "Invalid installer"):
            self.prepare()
        target = self.root / "synthetic-installer"
        target.write_bytes(original)
        path.unlink()
        path.symlink_to(target)
        with self.assertRaisesRegex(RELEASE.ReleaseError, "Invalid installer"):
            self.prepare()
        self.assert_no_generated_assets()
        self.assertEqual(self.client.downloads, [])

    def test_run_and_artifact_origin_guards_prevent_incorrect_provenance(self):
        original_run = copy.deepcopy(self.client.run)
        for key, value in [("run_attempt", 2), ("head_sha", "a" * 40), ("path", ".github/workflows/ci.yml"), ("status", "completed"), ("head_repository", {"full_name": "Other/PrismRelay"})]:
            with self.subTest(field=key):
                self.client.run = copy.deepcopy(original_run)
                self.client.run[key] = value
                with self.assertRaises(RELEASE.ReleaseError):
                    self.prepare()
                self.assert_no_generated_assets()
        self.client.run = original_run
        original_artifacts = copy.deepcopy(self.client.artifacts)
        for field, value in [("expired", True), ("digest", "sha256:bad"), ("id", True), ("id", 2), ("workflow_run", {"id": 43, "head_sha": self.sha}), ("workflow_run", {"id": 42, "head_sha": "a" * 40}), ("name", "unexpected-platform")]:
            with self.subTest(field=field, value=value):
                self.client.artifacts = copy.deepcopy(original_artifacts)
                self.client.artifacts[0][field] = value
                with self.assertRaises(RELEASE.ReleaseError):
                    self.prepare()
                self.assert_no_generated_assets()
        self.client.artifacts = original_artifacts[:-1]
        with self.assertRaises(RELEASE.ReleaseError):
            self.prepare()
        self.assertEqual(self.client.downloads, [])

    def test_archive_digest_and_actual_installer_bytes_are_verified(self):
        original = self.client.archives[1]
        self.client.archives[1] += b"Synthetic corrupt archive"
        with self.assertRaisesRegex(RELEASE.ReleaseError, "archive digest differs"):
            self.prepare()
        self.client.archives[1] = original
        local = self.installers / sorted(RELEASE.installer_names("windows-x64"))[0]
        local.write_bytes(b"Synthetic tampered installer")
        with self.assertRaisesRegex(RELEASE.ReleaseError, "artifact installer|verified artifact"):
            self.prepare()
        self.assert_no_generated_assets()

    def test_authenticated_archive_cannot_inject_unexpected_paths(self):
        self.client.archives[1] = self.client.archive({"../outside.exe": b"Synthetic installer"})
        self.client.artifacts[0]["digest"] = "sha256:" + hashlib.sha256(self.client.archives[1]).hexdigest()
        with self.assertRaisesRegex(RELEASE.ReleaseError, "installer names differ"):
            self.prepare()
        self.assertFalse((self.root / "outside.exe").exists())
        self.assert_no_generated_assets()


if __name__ == "__main__":
    unittest.main()
