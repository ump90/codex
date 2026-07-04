use core_test_support::responses::ev_completed;
use core_test_support::responses::ev_response_created;
use core_test_support::responses::mount_sse_once;
use core_test_support::responses::sse;
use core_test_support::test_codex::test_codex;
use tempfile::TempDir;
use wiremock::MockServer;

fn touch(path: &std::path::Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, [])?;
    Ok(())
}

fn create_git_for_windows_fixture(root: &std::path::Path) -> std::io::Result<()> {
    touch(&root.join("cmd").join("git.exe"))?;
    touch(&root.join("bin").join("bash.exe"))?;
    touch(&root.join("usr").join("bin").join("msys-2.0.dll"))?;
    Ok(())
}

fn toml_basic_string(value: &std::path::Path) -> String {
    let escaped = value
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn windows_path_to_git_bash_path(path: &std::path::Path) -> String {
    let path = path.to_string_lossy();
    let bytes = path.as_bytes();
    assert!(
        bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'\\' | b'/'),
        "expected drive-absolute Windows path, got {path}"
    );
    let drive = (bytes[0] as char).to_ascii_lowercase();
    let tail = path[3..].replace('\\', "/");
    if tail.is_empty() {
        format!("/{drive}/")
    } else {
        format!("/{drive}/{tail}")
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn configured_git_bash_renders_environment_context_shell() -> anyhow::Result<()> {
    let server = MockServer::start().await;
    let response_mock = mount_sse_once(
        &server,
        sse(vec![ev_response_created("resp1"), ev_completed("resp1")]),
    )
    .await;

    let fixture = TempDir::new()?;
    let git_root = fixture.path().join("Git");
    create_git_for_windows_fixture(&git_root)?;
    let bash_path = git_root.join("bin").join("bash.exe");
    let bash_path_for_config = bash_path.clone();
    let mut builder = test_codex().with_pre_build_hook(move |home| {
        let config = format!(
            "[windows]\ndefault_shell = \"git-bash\"\ngit_bash_path = {}\n",
            toml_basic_string(&bash_path_for_config)
        );
        std::fs::write(home.join("config.toml"), config).expect("write config.toml");
    });

    let test = builder.build(&server).await?;
    let expected_cwd = windows_path_to_git_bash_path(test.config.cwd.as_path());
    test.submit_turn("hello").await?;

    let request = response_mock.single_request();
    let environment_context = request
        .message_input_texts("user")
        .into_iter()
        .find(|text| text.starts_with("<environment_context>"))
        .expect("environment context should be sent");
    assert!(
        environment_context.contains("<shell>bash</shell>"),
        "expected Git Bash shell in environment context: {environment_context}"
    );
    assert!(
        environment_context.contains(&format!("<cwd>{expected_cwd}</cwd>")),
        "expected Git Bash cwd in environment context: {environment_context}"
    );
    assert!(
        !environment_context.contains("<shell>powershell</shell>"),
        "Git Bash config should not render PowerShell shell: {environment_context}"
    );

    Ok(())
}
