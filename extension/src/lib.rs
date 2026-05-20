use zed_extension_api::{
    self as zed, Architecture, ContextServerId, DownloadedFileType, Os, Project, Result,
    current_platform, latest_github_release,
};

struct AwsToolkitExtension;

impl zed::Extension for AwsToolkitExtension {
    fn new() -> Self {
        Self
    }

    fn context_server_command(
        &mut self,
        _context_server_id: &ContextServerId,
        _project: &Project,
    ) -> Result<zed::Command> {
        let (os, arch) = current_platform();

        let asset_name = match (os, arch) {
            (Os::Mac, Architecture::Aarch64) => "zed-aws-sidecar-macos-aarch64",
            (Os::Mac, Architecture::X8664) => "zed-aws-sidecar-macos-x86_64",
            (Os::Linux, Architecture::Aarch64) => "zed-aws-sidecar-linux-aarch64",
            (Os::Linux, Architecture::X8664) => "zed-aws-sidecar-linux-x86_64",
            _ => return Err(format!("unsupported platform: {os:?} {arch:?}")),
        };

        let release = latest_github_release(
            "clementGilardy/zed-aws-toolkit",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| format!("no asset {asset_name} in release {}", release.version))?;

        let binary_path = format!("zed-aws-sidecar-{}", release.version);

        zed::download_file(
            &asset.download_url,
            &binary_path,
            DownloadedFileType::Uncompressed,
        )?;
        zed::make_file_executable(&binary_path)?;

        Ok(zed::Command {
            command: binary_path,
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(AwsToolkitExtension);
