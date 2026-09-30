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
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("release_updater", pathlib.Path(__file__).with_name("update-release.py"))
RELEASE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RELEASE)


class FakeGithub:
    def __init__(self, source, sha, tag_sha):
        self.repository = "Example/prism-fixture"
        self.tag_sha = tag_sha
        self.run = {"id": 42, "workflow_id": 12, "head_branch": "main", "event": "push", "status": "completed", "conclusion": "success", "head_sha": sha, "run_attempt": 1, "path": ".github/workflows/ci.yml", "repository": {"full_name": self.repository}, "head_repository": {"full_name": self.repository}}
        self.artifacts = []
        self.archives = {}
        self.new_installers = {}
        for index, (target, platform) in enumerate(RELEASE.TARGETS.items(), 1):
            buffer = io.BytesIO()
            with zipfile.ZipFile(buffer, "w") as archive:
                for name in sorted(RELEASE.installer_names(platform)):
                    if name.endswith(".dmg"):
                        content = target.encode() + b"koly" + bytes(508)
                    else:
                        content = (b"MZ" if name.endswith("exe") else bytes.fromhex("d0cf11e0a1b11ae1")) + target.encode()
                    archive.writestr(name, content)
                    self.new_installers[name] = content
            self.archives[index] = buffer.getvalue()
            self.artifacts.append({"id": index, "name": RELEASE.artifact_name(target), "digest": "sha256:" + hashlib.sha256(buffer.getvalue()).hexdigest(), "expired": False, "workflow_run": {"id": 42, "head_sha": sha, "head_branch": "main"}})
        self.assets = {name: ("old " + name).encode() for name in RELEASE.INSTALLER_FILES + [RELEASE.SOURCE, RELEASE.LEGACY_WINDOWS_SOURCE, "BUILD_INFO.json"]}
        self.assets.update({name: (source / name).read_bytes() for name in ("LICENSE", "THIRD_PARTY_NOTICES.txt")})
        self.assets["SHA256SUMS.txt"] = "".join(hashlib.sha256(content).hexdigest() + "  " + name + "\n" for name, content in sorted(self.assets.items())).encode()
        self.ids = {name: index for index, name in enumerate(self.assets, 100)}
        self.next_id = 200
        self.body = "Original release body"
        self.immutable = False
        self.fail_upload = None
        self.fail_delete = None
        self.fail_edit = False
        self.corrupt_verification = False
        self.operations = []

    def api(self, endpoint):
        if endpoint == "actions/runs/42":
            return copy.deepcopy(self.run)
        if endpoint == "actions/workflows/ci.yml":
            return {"id": 12}
        if endpoint.startswith("actions/runs/42/artifacts?"):
            return {"artifacts": copy.deepcopy(self.artifacts)}
        if endpoint == "git/ref/tags/" + RELEASE.TAG:
            return {"object": {"type": "commit", "sha": self.tag_sha}}
        if endpoint == "releases/tags/" + RELEASE.TAG:
            return {"id": 7, "tag_name": RELEASE.TAG, "draft": False, "immutable": self.immutable, "body": self.body, "assets": [{"id": self.ids[name], "name": name, "size": len(content), "digest": "sha256:" + hashlib.sha256(content).hexdigest()} for name, content in self.assets.items()]}
        raise AssertionError(endpoint)

    def artifact(self, artifact_id, destination):
        destination.write_bytes(self.archives[artifact_id])

    def download(self, destination):
        for name, content in self.assets.items():
            (destination / name).write_bytes(content)
        if destination.name == "verified" and self.corrupt_verification:
            (destination / RELEASE.MAC_FILES[0]).write_bytes(b"corrupt download")

    def upload(self, path):
        name = path.name
        self.operations.append(("upload", name))
        self.assets.pop(name, None)
        self.ids.pop(name, None)
        if self.fail_upload == name:
            self.fail_upload = None
            raise RELEASE.UpdateError("Synthetic upload failure after deleting the original asset")
        self.assets[name] = path.read_bytes()
        self.ids[name] = self.next_id
        self.next_id += 1

    def delete(self, name):
        self.operations.append(("delete", name))
        self.assets.pop(name)
        self.ids.pop(name)
        if self.fail_delete == name:
            self.fail_delete = None
            raise RELEASE.UpdateError("Synthetic delete failure after removing the original asset")

    def edit(self, path):
        self.operations.append(("edit", path.name))
        self.body = path.read_bytes().decode("utf-8")
        if self.fail_edit:
            self.fail_edit = False
            raise RELEASE.UpdateError("Synthetic failure after updating the body")


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="prism-release-test-")
        self.root = pathlib.Path(self.temporary.name)
        self.source = self.root / "source"
        self.source.mkdir()
        files = {"package.json": '{"version":"1.0.0"}', "package-lock.json": '{"version":"1.0.0","packages":{"":{"version":"1.0.0"}}}', "src-tauri/tauri.conf.json": '{"version":"1.0.0"}', "src-tauri/Cargo.toml": '[package]\nname="prism-relay"\nversion="1.0.0"\n', "src-tauri/Cargo.lock": '[[package]]\nname="prism-relay"\nversion="1.0.0"\n', "docs/release-notes.md": '# Prism Relay 1.0.0\nSynthetic new release notes\n', "LICENSE": 'Synthetic GPL fixture\n', "THIRD_PARTY_NOTICES.txt": 'Synthetic dependency fixture\n'}
        for name, text in files.items():
            path = self.source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        self.git("init", "-b", "main")
        self.git("config", "user.name", "Synthetic Test")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "core.autocrlf", "false")
        self.git("add", ".")
        self.git("commit", "-m", "Synthetic original")
        self.tag_sha = self.git("rev-parse", "HEAD").decode().strip()
        self.git("tag", RELEASE.TAG)
        (self.source / "fixture.txt").write_text("Synthetic application revision\n")
        self.git("add", ".")
        self.git("commit", "-m", "Synthetic application revision")
        self.sha = self.git("rev-parse", "HEAD").decode().strip()
        self.git("update-ref", "refs/remotes/origin/main", self.sha)
        self.tag_patch = patch.object(RELEASE, "TAG_COMMIT", self.tag_sha)
        self.tag_patch.start()
        self.client = FakeGithub(self.source, self.sha, self.tag_sha)
        self.state = self.root / "state"

    def tearDown(self):
        self.tag_patch.stop()
        self.temporary.cleanup()

    def git(self, *arguments):
        return subprocess.check_output(["git", "-C", str(self.source), *arguments], stderr=subprocess.PIPE)

    def prepared(self):
        RELEASE.inspect(self.client, self.state, 42)
        RELEASE.prepare(self.client, self.state, self.source)
        self.assertEqual(self.client.operations, [])

    def test_success_updates_all_platforms_preserves_tag_licenses_and_matches_unified_source(self):
        old = dict(self.client.assets)
        self.prepared()
        RELEASE.publish(self.client, self.state, "99")
        for name in ("LICENSE", "THIRD_PARTY_NOTICES.txt"):
            self.assertEqual(self.client.assets[name], old[name])
        for name, content in self.client.new_installers.items():
            self.assertEqual(self.client.assets[name], content)
            self.assertNotEqual(self.client.assets[name], old[name])
        self.assertEqual(len(self.client.new_installers), 6)
        self.assertEqual(len(self.client.assets), 11)
        self.assertNotIn(RELEASE.LEGACY_WINDOWS_SOURCE, self.client.assets)
        self.assertEqual(self.git("rev-parse", RELEASE.TAG).decode().strip(), self.tag_sha)
        with tarfile.open(fileobj=io.BytesIO(self.client.assets[RELEASE.SOURCE]), mode="r:gz") as archive:
            self.assertEqual(archive.pax_headers["comment"], self.sha)
            members = {entry.name.removeprefix("prism-relay-v1.0.0/"): entry for entry in archive if entry.isfile()}
            expected = self.git("ls-tree", "-r", "--name-only", self.sha).decode().splitlines()
            self.assertEqual(set(members), set(expected))
            for name, entry in members.items():
                self.assertEqual(archive.extractfile(entry).read(), self.git("show", self.sha + ":" + name))
                self.assertEqual(entry.mode, 0o664)
        info = json.loads(self.client.assets["BUILD_INFO.json"])
        self.assertEqual(info["tagCommit"], self.tag_sha)
        self.assertEqual(info["backupArtifactId"], 99)
        for platform, files in (("windows", RELEASE.WINDOWS_FILES), ("macOS", RELEASE.MAC_FILES)):
            self.assertEqual(info[platform]["sourceCommit"], self.sha)
            self.assertEqual(info[platform]["sourceArchive"], RELEASE.SOURCE)
            self.assertEqual(info[platform]["ciRunId"], 42)
            self.assertEqual(info[platform]["ciRunAttempt"], 1)
            self.assertEqual(len(info[platform]["artifacts"]), 2)
            self.assertEqual(set(info[platform]["sha256"]), set(files + [RELEASE.SOURCE]))
            for name, checksum in info[platform]["sha256"].items():
                self.assertEqual(checksum, hashlib.sha256(self.client.assets[name]).hexdigest())
        names = {line.split("  ")[1] for line in self.client.assets["SHA256SUMS.txt"].decode().splitlines()}
        self.assertEqual(names, set(self.client.assets) - {"SHA256SUMS.txt"})
        for line in self.client.assets["SHA256SUMS.txt"].decode().splitlines():
            checksum, name = line.split("  ")
            self.assertEqual(checksum, hashlib.sha256(self.client.assets[name]).hexdigest())

    def test_ci_and_release_guards_prevent_any_upload(self):
        cases = [("head_branch", "other"), ("event", "pull_request"), ("status", "in_progress"), ("conclusion", "failure"), ("workflow_id", 13), ("path", ".github/workflows/release.yml"), ("head_sha", "invalid"), ("head_repository", {"full_name": "Other/fork"})]
        original = copy.deepcopy(self.client.run)
        for field, value in cases:
            with self.subTest(field=field):
                self.client.run = copy.deepcopy(original)
                self.client.run[field] = value
                with self.assertRaises(RELEASE.UpdateError):
                    RELEASE.inspect(self.client, self.state, 42)
                self.assertFalse(self.state.exists())
        self.client.run = original
        self.client.immutable = True
        with self.assertRaises(RELEASE.UpdateError):
            RELEASE.inspect(self.client, self.state, 42)
        self.assertEqual(self.client.operations, [])

    def test_artifact_origin_expiry_and_digest_are_verified(self):
        original = copy.deepcopy(self.client.artifacts)
        for index in range(4):
            for failure in ("expired", "source", "missing", "duplicate"):
                with self.subTest(target=index, failure=failure):
                    self.client.artifacts = copy.deepcopy(original)
                    if failure == "expired":
                        self.client.artifacts[index]["expired"] = True
                    elif failure == "source":
                        self.client.artifacts[index]["workflow_run"]["head_sha"] = "0" * 40
                    elif failure == "missing":
                        self.client.artifacts.pop(index)
                    else:
                        self.client.artifacts.append(copy.deepcopy(self.client.artifacts[index]))
                    with self.assertRaises(RELEASE.UpdateError):
                        RELEASE.inspect(self.client, self.state, 42)
                    self.assertFalse(self.state.exists())
        self.client.artifacts = original
        RELEASE.inspect(self.client, self.state, 42)
        self.client.archives[3] += b"changed"
        with self.assertRaises(RELEASE.UpdateError):
            RELEASE.prepare(self.client, self.state, self.source)
        self.assertEqual(self.client.operations, [])

    def test_artifact_members_and_dmg_trailer_are_checked(self):
        original_archives = dict(self.client.archives)
        original_artifacts = copy.deepcopy(self.client.artifacts)
        for case in ("unexpected", "symlink", "invalid-dmg"):
            with self.subTest(case=case):
                self.client.archives = dict(original_archives)
                self.client.artifacts = copy.deepcopy(original_artifacts)
                buffer = io.BytesIO()
                with zipfile.ZipFile(buffer, "w") as archive:
                    name = RELEASE.MAC_FILES[0]
                    if case == "unexpected":
                        name = "../" + name
                    entry = zipfile.ZipInfo(name)
                    if case == "symlink":
                        entry.create_system = 3
                        entry.external_attr = 0o120777 << 16
                    content = b"bad" if case == "invalid-dmg" else b"koly" + bytes(508)
                    archive.writestr(entry, content)
                self.client.archives[3] = buffer.getvalue()
                self.client.artifacts[2]["digest"] = "sha256:" + hashlib.sha256(buffer.getvalue()).hexdigest()
                state = self.root / ("state-" + case)
                RELEASE.inspect(self.client, state, 42)
                with self.assertRaises(RELEASE.UpdateError):
                    RELEASE.prepare(self.client, state, self.source)
                self.assertEqual(self.client.operations, [])

    def test_partial_upload_failure_restores_clobbered_installers_and_source(self):
        old = dict(self.client.assets)
        body = self.client.body
        self.prepared()
        self.client.fail_upload = RELEASE.MAC_FILES[1]
        with self.assertRaisesRegex(RELEASE.UpdateError, "were restored"):
            RELEASE.publish(self.client, self.state, "99")
        self.assertEqual(self.client.assets, old)
        self.assertEqual(self.client.body, body)
        self.assertTrue((self.state / "backup/assets/SHA256SUMS.txt").is_file())

    def test_deleted_legacy_source_is_restored_when_delete_fails(self):
        old = dict(self.client.assets)
        body = self.client.body
        self.prepared()
        self.client.fail_delete = RELEASE.LEGACY_WINDOWS_SOURCE
        with self.assertRaisesRegex(RELEASE.UpdateError, "were restored"):
            RELEASE.publish(self.client, self.state, "99")
        self.assertEqual(self.client.assets, old)
        self.assertEqual(self.client.body, body)
        self.assertIn(("delete", RELEASE.LEGACY_WINDOWS_SOURCE), self.client.operations)
        self.assertIn(("upload", RELEASE.LEGACY_WINDOWS_SOURCE), self.client.operations)

    def test_existing_unified_release_can_be_updated_without_legacy_source(self):
        self.client.assets.pop(RELEASE.LEGACY_WINDOWS_SOURCE)
        self.client.ids.pop(RELEASE.LEGACY_WINDOWS_SOURCE)
        self.client.assets["SHA256SUMS.txt"] = "".join(hashlib.sha256(content).hexdigest() + "  " + name + "\n" for name, content in sorted(self.client.assets.items()) if name != "SHA256SUMS.txt").encode()
        self.prepared()
        RELEASE.publish(self.client, self.state, "99")
        self.assertNotIn(RELEASE.LEGACY_WINDOWS_SOURCE, self.client.assets)
        self.assertFalse(any(operation[0] == "delete" for operation in self.client.operations))

    def test_checksum_upload_failure_restores_every_asset(self):
        old = dict(self.client.assets)
        self.prepared()
        self.client.fail_upload = "SHA256SUMS.txt"
        with self.assertRaisesRegex(RELEASE.UpdateError, "were restored"):
            RELEASE.publish(self.client, self.state, "99")
        self.assertEqual(self.client.assets, old)

    def test_body_failure_restores_old_body_and_assets(self):
        old = dict(self.client.assets)
        body = self.client.body
        self.prepared()
        self.client.fail_edit = True
        with self.assertRaisesRegex(RELEASE.UpdateError, "were restored"):
            RELEASE.publish(self.client, self.state, "99")
        self.assertEqual(self.client.assets, old)
        self.assertEqual(self.client.body, body)

    def test_download_verification_failure_restores_original_release(self):
        old = dict(self.client.assets)
        self.prepared()
        self.client.corrupt_verification = True
        with self.assertRaisesRegex(RELEASE.UpdateError, "were restored"):
            RELEASE.publish(self.client, self.state, "99")
        self.assertEqual(self.client.assets, old)

    def test_missing_backup_and_changed_release_or_staging_abort_before_upload(self):
        self.prepared()
        with self.assertRaises(RELEASE.UpdateError):
            RELEASE.publish(self.client, self.state, "")
        self.client.body = "Changed elsewhere"
        with self.assertRaises(RELEASE.UpdateError):
            RELEASE.publish(self.client, self.state, "99")
        self.client.body = "Original release body"
        (self.state / "output" / RELEASE.WINDOWS_FILES[0]).write_bytes(b"Changed staging")
        with self.assertRaises(RELEASE.UpdateError):
            RELEASE.publish(self.client, self.state, "99")
        self.assertEqual(self.client.operations, [])

    def test_modified_backup_body_and_prepared_notes_abort_before_upload(self):
        self.prepared()
        body = self.state / "backup/release-body.md"
        original = body.read_bytes()
        body.write_text("Changed backup")
        with self.assertRaisesRegex(RELEASE.UpdateError, "body backup changed"):
            RELEASE.publish(self.client, self.state, "99")
        body.write_bytes(original)
        (self.state / "release-notes.md").write_text("Changed notes")
        with self.assertRaisesRegex(RELEASE.UpdateError, "notes changed"):
            RELEASE.publish(self.client, self.state, "99")
        self.assertEqual(self.client.operations, [])

    def crlf_release(self):
        notes = b"# Prism Relay 1.0.0\r\nSynthetic CRLF release notes\r\n"
        (self.source / "docs/release-notes.md").write_bytes(notes)
        self.git("add", ".")
        self.git("commit", "-m", "Synthetic CRLF notes")
        self.sha = self.git("rev-parse", "HEAD").decode().strip()
        self.git("update-ref", "refs/remotes/origin/main", self.sha)
        self.client = FakeGithub(self.source, self.sha, self.tag_sha)
        self.client.body = "Original CRLF release body\r\nSecond line\r\n"
        return notes

    def test_crlf_release_body_and_notes_are_preserved_on_success(self):
        notes = self.crlf_release()
        body = self.client.body.encode("utf-8")
        self.prepared()
        self.assertEqual((self.state / "backup/release-body.md").read_bytes(), body)
        self.assertEqual((self.state / "release-notes.md").read_bytes(), notes)
        RELEASE.publish(self.client, self.state, "99")
        self.assertEqual(self.client.body.encode("utf-8"), notes)

    def test_crlf_original_body_is_restored_after_body_update_failure(self):
        self.crlf_release()
        old = dict(self.client.assets)
        body = self.client.body.encode("utf-8")
        self.prepared()
        self.client.fail_edit = True
        with self.assertRaisesRegex(RELEASE.UpdateError, "were restored"):
            RELEASE.publish(self.client, self.state, "99")
        self.assertEqual(self.client.assets, old)
        self.assertEqual(self.client.body.encode("utf-8"), body)


if __name__ == "__main__":
    unittest.main()
