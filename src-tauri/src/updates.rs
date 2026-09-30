use serde::{Deserialize, Serialize};

const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/Shake1227/PrismRelay/releases/latest";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    current_version: String,
    latest_version: String,
    update_available: bool,
    release_url: String,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
}

fn version_parts(value: &str) -> Option<(u64, u64, u64)> {
    let parts = value
        .strip_prefix('v')
        .unwrap_or(value)
        .split('.')
        .map(|part| {
            if part.is_empty()
                || !part.bytes().all(|byte| byte.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
            {
                None
            } else {
                part.parse::<u64>().ok()
            }
        })
        .collect::<Option<Vec<_>>>()?;
    if parts.len() != 3 {
        return None;
    }
    Some((parts[0], parts[1], parts[2]))
}

fn official_url(url: &reqwest::Url, host: &str) -> bool {
    url.scheme() == "https"
        && url.host_str() == Some(host)
        && url.username().is_empty()
        && url.password().is_none()
        && url.port().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}

fn project_path(owner: &str, repository: &str) -> bool {
    owner.eq_ignore_ascii_case("Shake1227")
        && (repository.eq_ignore_ascii_case("PrismRelay")
            || repository.eq_ignore_ascii_case("prism-relay"))
}

fn release_url(value: &str, tag: &str) -> Option<String> {
    let url = reqwest::Url::parse(value).ok()?;
    if !official_url(&url, "github.com") {
        return None;
    }
    let path = url.path_segments()?.collect::<Vec<_>>();
    match path.as_slice() {
        [owner, repository, "releases", "tag", release_tag]
            if project_path(owner, repository) && *release_tag == tag =>
        {
            Some(url.into())
        }
        _ => None,
    }
}

fn release_api_url(url: &reqwest::Url) -> bool {
    if !official_url(url, "api.github.com") {
        return false;
    }
    let path = url
        .path_segments()
        .map(|parts| parts.collect::<Vec<_>>())
        .unwrap_or_default();
    matches!(
        path.as_slice(),
        ["repos", owner, repository, "releases", "latest"]
            if project_path(owner, repository)
    )
}

fn update_info(current_version: &str, release: Release) -> Result<UpdateInfo, String> {
    let latest = version_parts(&release.tag_name).ok_or("更新情報の形式が無効です。")?;
    let current = version_parts(current_version).ok_or("アプリのバージョン情報が無効です。")?;
    let url = release_url(&release.html_url, &release.tag_name)
        .ok_or("更新情報のリンクを確認できません。")?;
    Ok(UpdateInfo {
        current_version: current_version.into(),
        latest_version: release
            .tag_name
            .strip_prefix('v')
            .unwrap_or(&release.tag_name)
            .into(),
        update_available: latest > current,
        release_url: url,
    })
}

pub fn check() -> Result<UpdateInfo, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() < 5 && release_api_url(attempt.url()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
        .map_err(|_| "更新情報に接続できません。".to_string())?;
    let release: Release = client
        .get(LATEST_RELEASE_API)
        .header("User-Agent", "Prism-Relay-update-check")
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|_| {
            "更新情報を取得できません。GitHubのReleaseページを確認してください。".to_string()
        })?;
    update_info(env!("CARGO_PKG_VERSION"), release)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.into(),
            html_url: format!("https://github.com/Shake1227/PrismRelay/releases/tag/{tag}"),
        }
    }

    #[test]
    fn accepts_canonical_github_response() {
        let response = r#"{"tag_name":"v1.0.0","html_url":"https://github.com/Shake1227/PrismRelay/releases/tag/v1.0.0","draft":false,"prerelease":false}"#;
        let info = update_info("1.0.0", serde_json::from_str(response).unwrap()).unwrap();
        assert_eq!(info.current_version, "1.0.0");
        assert_eq!(info.latest_version, "1.0.0");
        assert!(!info.update_available);
        assert_eq!(info.release_url, release("v1.0.0").html_url);
    }

    #[test]
    fn allows_canonical_and_historical_project_names() {
        for repository in ["PrismRelay", "prism-relay", "prismrelay"] {
            let url = format!("https://github.com/SHake1227/{repository}/releases/tag/v1.0.0");
            assert!(release_url(&url, "v1.0.0").is_some(), "{url}");
        }
    }

    #[test]
    fn rejects_untrusted_or_mismatched_release_links() {
        for url in [
            "https://github.com.evil.example/Shake1227/PrismRelay/releases/tag/v1.0.0",
            "https://github.com@evil.example/Shake1227/PrismRelay/releases/tag/v1.0.0",
            "https://evil.example@github.com/Shake1227/PrismRelay/releases/tag/v1.0.0",
            "http://github.com/Shake1227/PrismRelay/releases/tag/v1.0.0",
            "https://github.com:444/Shake1227/PrismRelay/releases/tag/v1.0.0",
            "https://github.com/OtherOwner/PrismRelay/releases/tag/v1.0.0",
            "https://github.com/Shake1227/OtherRepo/releases/tag/v1.0.0",
            "https://github.com/Shake1227/PrismRelay.evil/releases/tag/v1.0.0",
            "https://github.com/Shake1227/PrismRelay/releases/tag/v1.0.1",
            "https://github.com/Shake1227/PrismRelay/releases/tag/v1.0.0/extra",
            "https://github.com/Shake1227/PrismRelay/releases/tag/v1.0.0?redirect=https://evil.example",
            "https://github.com/Shake1227/PrismRelay/releases/tag/v1.0.0#evil",
            "https://github.com/Shake1227/PrismRelay/releases/latest",
            "https://github.com/Shake1227/%50rismRelay/releases/tag/v1.0.0",
        ] {
            assert!(release_url(url, "v1.0.0").is_none(), "{url}");
            let result = update_info(
                "1.0.0",
                Release { tag_name: "v1.0.0".into(), html_url: url.into() },
            );
            assert_eq!(result.unwrap_err(), "更新情報のリンクを確認できません。");
        }
    }

    #[test]
    fn redirects_stay_on_official_project_release_api() {
        for url in [
            LATEST_RELEASE_API,
            "https://api.github.com/repos/SHake1227/prism-relay/releases/latest",
        ] {
            assert!(release_api_url(&reqwest::Url::parse(url).unwrap()));
        }
        for url in [
            "http://api.github.com/repos/Shake1227/PrismRelay/releases/latest",
            "https://api.github.com.evil.example/repos/Shake1227/PrismRelay/releases/latest",
            "https://api.github.com/repos/OtherOwner/PrismRelay/releases/latest",
            "https://api.github.com/repos/Shake1227/OtherRepo/releases/latest",
            "https://api.github.com/repos/Shake1227/PrismRelay/releases/latest?redirect=evil",
            "https://github.com/Shake1227/PrismRelay/releases/latest",
        ] {
            assert!(
                !release_api_url(&reqwest::Url::parse(url).unwrap()),
                "{url}"
            );
        }
    }

    #[test]
    fn compares_stable_versions_numerically() {
        assert!(version_parts("v0.10.0") > version_parts("0.2.0"));
        for tag in ["v1.0.1", "v1.1.0", "v2.0.0"] {
            assert!(update_info("1.0.0", release(tag)).unwrap().update_available);
        }
        for tag in ["v1.0.0", "v0.99.99"] {
            assert!(!update_info("1.0.0", release(tag)).unwrap().update_available);
        }
    }

    #[test]
    fn rejects_invalid_versions_and_incomplete_responses() {
        for value in [
            "latest",
            "1.0",
            "1.0.0.1",
            "vv1.0.0",
            "+1.0.0",
            "1.00.0",
            "1.0.0-rc.1",
            "1.0.0 ",
            "18446744073709551616.0.0",
        ] {
            assert_eq!(version_parts(value), None, "{value}");
            assert_eq!(
                update_info("1.0.0", release(value)).unwrap_err(),
                "更新情報の形式が無効です。"
            );
        }
        for json in [
            "{}",
            r#"{"tag_name":"v1.0.0"}"#,
            r#"{"tag_name":1,"html_url":"https://github.com"}"#,
        ] {
            assert!(serde_json::from_str::<Release>(json).is_err());
        }
    }
}
