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
        // Zed sets the working directory to the extension bundle directory at runtime
        let sidecar_path = std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join("zed-aws-sidecar");

        Ok(zed::Command {
            command: sidecar_path.to_string_lossy().to_string(),
            args: vec![],
            env: vec![],
        })
    }
}

zed::register_extension!(AwsToolkitExtension);
