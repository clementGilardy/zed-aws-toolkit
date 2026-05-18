use zed_extension_api::{self as zed, ContextServerId, Project, Result};

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
        let sidecar_path = std::env::current_exe()
            .map_err(|e| e.to_string())?
            .parent()
            .ok_or("no parent dir")?
            .join("zed-aws-sidecar");

        Ok(zed::Command {
            command: sidecar_path.to_string_lossy().to_string(),
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(AwsToolkitExtension);
