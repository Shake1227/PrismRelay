use serde::{Deserialize, Serialize};

#[derive(Serialize)]
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
        .trim_start_matches('v')
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    if parts.len() != 3 {
        return None;
    }
    Some((parts[0], parts[1], parts[2]))
}

pub fn check() -> Result<UpdateInfo, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|_| "更新情報に接続できません。".to_string())?;
    let release: Release = client
        .get("https://api.github.com/repos/SHake1227/prism-relay/releases/latest")
        .header("User-Agent", "Prism-Relay-update-check")
        .header("Accept", "application/vnd.github+json")
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|_| {
            "更新情報を取得できません。GitHubのReleaseページを確認してください。".to_string()
        })?;
    let latest = version_parts(&release.tag_name).ok_or("更新情報の形式が無効です。")?;
    if !release
        .html_url
        .starts_with("https://github.com/SHake1227/prism-relay/releases/")
        && !release
            .html_url
            .starts_with("https://github.com/Shake1227/prism-relay/releases/")
    {
        return Err("更新情報のリンクを確認できません。".into());
    }
    Ok(UpdateInfo {
        current_version: env!("CARGO_PKG_VERSION").into(),
        latest_version: release.tag_name.trim_start_matches('v').into(),
        update_available: latest > version_parts(env!("CARGO_PKG_VERSION")).unwrap_or_default(),
        release_url: release.html_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn semantic_version_comparison() {
        assert!(version_parts("v0.10.0") > version_parts("0.2.0"));
        assert_eq!(version_parts("latest"), None);
        assert_eq!(version_parts("1.0"), None);
    }
}
