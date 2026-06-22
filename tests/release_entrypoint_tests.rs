use std::fs;
use std::path::Path;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn repo_file(path: &str) -> String {
    let full_path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    fs::read_to_string(&full_path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", full_path.display()))
}

fn repo_path(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn assert_no_legacy_docker_paths(label: &str, contents: &str) {
    for legacy in [
        "/app/data",
        "/app/backup",
        "/app/mod",
        "/app/steamcmd",
        "/app/dst-dedicated-server",
        "/app/dst-db",
        "/app/password.txt",
        "/app/first",
        "/app/dst-admin-rust.log",
        "dst-admin-go:",
    ] {
        assert!(
            !contents.contains(legacy),
            "{label} still documents legacy Docker path or image: {legacy}"
        );
    }
}

fn root_asset_paths_from_index(index: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for marker in ["src=\"/", "href=\"/"] {
        for fragment in index.split(marker).skip(1) {
            if let Some((path, _)) = fragment.split_once('"') {
                paths.push(path.to_string());
            }
        }
    }
    paths
}

#[cfg(unix)]
fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents)
        .unwrap_or_else(|error| panic!("failed to write executable {}: {error}", path.display()));
    let mut permissions = fs::metadata(path)
        .unwrap_or_else(|error| panic!("failed to stat {}: {error}", path.display()))
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .unwrap_or_else(|error| panic!("failed to chmod {}: {error}", path.display()));
}

#[cfg(unix)]
fn prepend_path(command: &mut Command, bin_dir: &Path) {
    let old_path = std::env::var("PATH").unwrap_or_default();
    command.env("PATH", format!("{}:{old_path}", bin_dir.display()));
}

#[cfg(unix)]
fn install_fake_rust_tools(bin_dir: &Path, installed_target: &str, host_target: &str) {
    write_executable(
        &bin_dir.join("rustup"),
        &format!("#!/bin/sh\nif [ \"$1\" = \"target\" ]; then echo \"{installed_target}\"; fi\n"),
    );
    write_executable(
        &bin_dir.join("rustc"),
        &format!("#!/bin/sh\nprintf 'host: {host_target}\\n'\n"),
    );
    let cargo_script = r#"#!/bin/sh
target=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--target" ]; then
    shift
    target="$1"
  fi
  shift
done
case "$target" in
  aarch64-unknown-linux-gnu)
    if [ -z "$CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER" ]; then
      echo "missing cargo aarch64 linker env" >&2
      exit 42
    fi
    if [ -n "$EXPECTED_AARCH64_LINKER" ] && [ "$CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER" != "$EXPECTED_AARCH64_LINKER" ]; then
      echo "wrong cargo aarch64 linker env" >&2
      exit 43
    fi
    ;;
  x86_64-unknown-linux-gnu)
    if [ -z "$CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER" ]; then
      echo "missing cargo x86_64 linker env" >&2
      exit 42
    fi
    ;;
esac
mkdir -p "target/$target/release"
case "$target" in
  *windows*) touch "target/$target/release/dst-admin-rust.exe" ;;
  *) touch "target/$target/release/dst-admin-rust" ;;
esac
"#;
    write_executable(&bin_dir.join("cargo"), cargo_script);
}

#[cfg(unix)]
fn install_fake_rust_tools_without_target(bin_dir: &Path, host_target: &str) {
    install_fake_rust_tools(bin_dir, "wasm32-unknown-unknown", host_target);
}

#[test]
fn docker_and_install_entrypoints_execute_dst_admin_rust() {
    let checked_files = [
        "Dockerfile",
        "docker-entrypoint.sh",
        "script/docker-build-mac/Dockerfile",
        "script/docker-build-mac/docker-entrypoint.sh",
        "static/script/dst-go.sh",
        "build_linux.sh",
        "build_window.sh",
        "docs/multiServer.md",
    ];

    for path in checked_files {
        let contents = repo_file(path);
        assert!(
            contents.contains("dst-admin-rust"),
            "{path} should reference the Rust binary"
        );
        assert!(
            !contents.contains("exec ./dst-admin-go")
                && !contents.contains("COPY dst-admin-go ")
                && !contents.contains("nohup ./dst-admin-go")
                && !contents.contains("pgrep dst-admin-go")
                && !contents.contains("pkill dst-admin-go"),
            "{path} still launches or manages the old Go binary"
        );
        for line in contents.lines().map(str::trim) {
            assert!(
                !line.starts_with("go build")
                    && !line.starts_with("go run main.go")
                    && !line.starts_with("GOOS=")
                    && !line.starts_with("GOARCH="),
                "{path} still contains old Go build command: {line}"
            );
        }
    }
}

#[test]
fn docker_platforms_match_the_release_binary_targets() {
    let linux_dockerfile = repo_file("Dockerfile");
    assert!(
        linux_dockerfile.contains("FROM --platform=linux/amd64 ubuntu:20.04"),
        "root Dockerfile should be pinned to linux/amd64 to match the default Linux binary"
    );

    let mac_arm_dockerfile = repo_file("script/docker-build-mac/Dockerfile");
    assert!(
        mac_arm_dockerfile.contains("FROM --platform=linux/arm64 ubuntu:22.04"),
        "mac arm Dockerfile should be pinned to linux/arm64"
    );
    let mac_arm_release_dockerfile = repo_file("scripts/docker-build-mac/Dockerfile");
    assert!(
        mac_arm_release_dockerfile.contains("FROM --platform=linux/arm64 ubuntu:22.04"),
        "release mac arm Dockerfile should be pinned to linux/arm64"
    );

    let mac_arm_notes = repo_file("script/docker-build-mac/dst-mac-arm64-env-install.md");
    assert!(
        mac_arm_notes.contains("RUST_TARGET=aarch64-unknown-linux-gnu ./build_linux.sh"),
        "mac arm Docker docs should explain how to build a matching Rust binary"
    );
}

#[test]
fn install_docs_build_local_rust_docker_image() {
    let install_doc = repo_file("docs/install.md");
    assert!(
        install_doc.contains("./build_linux.sh"),
        "install docs should build the Rust binary before Docker images"
    );
    assert!(
        install_doc.contains("docker build -t dst-admin-rust:local ."),
        "install docs should build a local Rust Docker image"
    );
    assert!(
        install_doc.contains("dst-admin-rust:local"),
        "install docs should run the local Rust Docker image"
    );
    assert!(
        !install_doc.contains("docker pull") && !install_doc.contains("dst-admin-go:1.3.1"),
        "install docs should not direct users to the legacy Go Docker image"
    );
    for legacy in [
        "/app/backup",
        "/app/mod",
        "/app/steamcmd",
        "/app/dst-dedicated-server",
        "/app/dst-db",
        "/app/password.txt",
        "/app/first",
        "/app/dst-admin-rust.log",
    ] {
        assert!(
            !install_doc.contains(legacy),
            "install docs should not document legacy Docker path: {legacy}"
        );
    }
}

#[test]
fn docker_publish_script_builds_and_pushes_rust_image() {
    let script = repo_file("docker_build.sh");
    assert!(
        script.contains("./build_linux.sh"),
        "Docker publish script should build the Rust binary first"
    );
    assert!(
        script.contains("IMAGE_NAME=${IMAGE_NAME:-yimuu/dst-panel}"),
        "Docker publish script should tag the Rust image"
    );
    assert!(
        !script.contains("dst-admin-go:$TAG"),
        "Docker publish script should not publish the legacy Go image"
    );
}

#[test]
fn docker_entrypoint_maps_config_data_dir_to_data_volume() {
    let entrypoint = repo_file("docker-entrypoint.sh");
    assert!(entrypoint.contains("DATA_DIR=\"/data\""));
    assert!(entrypoint.contains("cd \"$DATA_DIR\""));
    assert!(entrypoint.contains("exec \"$APP_DIR/dst-admin-rust\""));
    assert!(
        entrypoint.contains("dataDir: \".\""),
        "Docker entrypoint should make config.yml open /data/dst-db directly"
    );
    assert!(
        entrypoint.contains("cp -a \"$APP_DIR/dist/.\" \"$DATA_DIR/dist/\""),
        "Docker entrypoint should refresh packaged frontend assets on every container start"
    );
    assert!(
        entrypoint.contains("cp -a \"$APP_DIR/static/.\" \"$DATA_DIR/static/\""),
        "Docker entrypoint should refresh packaged static assets on every container start"
    );
    assert!(
        !entrypoint.contains("if [ ! -d \"$DATA_DIR/dist\" ]"),
        "Docker entrypoint should not leave stale frontend assets after image upgrades"
    );
}

#[test]
fn docker_context_contains_dist_directory_for_clean_checkout_builds() {
    let index_path = repo_path("dist/index.html");
    assert!(
        index_path.is_file(),
        "Dockerfile copies dist, so a clean checkout must include a dist directory"
    );
    let index = fs::read_to_string(&index_path).expect("read dist/index.html");
    assert!(
        index.contains("assets/index-") && index.contains("<script type=\"module\""),
        "dist/index.html should be the production frontend shell, not a placeholder"
    );
    let asset_paths = root_asset_paths_from_index(&index);
    assert!(
        !asset_paths.is_empty(),
        "dist/index.html should reference compiled frontend assets"
    );
    for asset_path in asset_paths {
        assert!(
            repo_path(&format!("dist/{asset_path}")).is_file(),
            "dist/index.html references a missing asset: /{asset_path}"
        );
    }
    assert!(
        repo_path("dist/assets").is_dir(),
        "production frontend assets should be checked in for clean Docker builds"
    );
    assert!(
        repo_path("dist/misc").is_dir(),
        "production frontend misc data should be checked in for clean Docker builds"
    );
    assert!(
        !repo_path("dist/mockServiceWorker.js").exists() && !index.contains("mockServiceWorker"),
        "release dist must not use the mocked preview frontend"
    );
}

#[test]
fn frontend_dist_docker_references_use_rust_data_volume_layout() {
    let compose = repo_file("dist/misc/Docker-compose.md");
    assert!(compose.contains("dst-admin-rust"));
    assert!(compose.contains("yimuu/dst-panel"));
    assert!(compose.contains("- ${PWD}/dstsave:/data"));
    assert_no_legacy_docker_paths("dist/misc/Docker-compose.md", &compose);

    let mut js_bundle = String::new();
    for entry in fs::read_dir(repo_path("dist/assets")).expect("read dist/assets") {
        let entry = entry.expect("read dist asset entry");
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("js") {
            js_bundle.push_str(
                &fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("read {}: {error}", path.display())),
            );
        }
    }
    assert!(
        js_bundle.contains("Docker 路径参考"),
        "frontend bundle should include the Docker path reference drawer"
    );
    assert!(js_bundle.contains("/data/backup"));
    assert!(js_bundle.contains("/data/dst-dedicated-server"));
    assert!(js_bundle.contains("yimuu/dst-panel"));
    assert_no_legacy_docker_paths("dist/assets/*.js", &js_bundle);
}

#[test]
fn contributor_docs_describe_rust_commands_after_cutover() {
    let docs = repo_file("CLAUDE.md");
    assert!(docs.contains("cargo run --bin dst-admin-rust"));
    assert!(docs.contains("cargo test --locked"));
    assert!(docs.contains("./build_linux.sh"));
    assert!(docs.contains("./build_window.sh"));
    for legacy in [
        "go mod tidy",
        "go run cmd/server/main.go",
        "go build",
        "GOOS=",
        "GOARCH=",
        "cmd/server/main.go",
        "internal/api",
        "GORM",
        "Written in Go",
    ] {
        assert!(
            !docs.contains(legacy),
            "CLAUDE.md still contains legacy Go guidance: {legacy}"
        );
    }
}

#[test]
fn docker_release_docs_use_data_volume_layout() {
    let entrypoint = repo_file("scripts/docker/docker-entrypoint.sh");
    assert!(entrypoint.contains("DATA_DIR=\"/data\""));
    assert!(entrypoint.contains("APP_DIR=\"/app\""));
    assert!(entrypoint.contains("cd \"$DATA_DIR\""));
    assert!(entrypoint.contains("exec \"$APP_DIR/dst-admin-rust\""));
    assert!(!entrypoint.contains(">> /app/password.txt"));
    assert!(!entrypoint.contains("ln -sf \"$data_db_file\" /app/dst-db"));
    assert!(!entrypoint.contains("ln -sf \"$password_file\""));
    assert!(!entrypoint.contains("ln -sf \"$first_file\""));
    assert!(
        entrypoint.contains("cp -a \"$APP_DIR/dist/.\" \"$DATA_DIR/dist/\""),
        "release Docker entrypoint should refresh packaged frontend assets on every start"
    );
    assert!(
        entrypoint.contains("cp -a \"$APP_DIR/static/.\" \"$DATA_DIR/static/\""),
        "release Docker entrypoint should refresh packaged static assets on every start"
    );
    assert!(
        !entrypoint.contains("if [ ! -d \"$DATA_DIR/dist\" ]"),
        "release Docker entrypoint should not leave stale frontend assets after image upgrades"
    );

    let docs = repo_file("scripts/docker/README.md");
    assert!(docs.contains("-v ~/dstsave:/data"));
    assert!(docs.contains("dataDir: \".\""));
    assert_no_legacy_docker_paths("scripts/docker/README.md", &docs);
}

#[test]
fn docker_dst_config_uses_data_volume_for_klei_and_game_paths() {
    for path in ["docker_dst_config", "scripts/docker/docker_dst_config"] {
        let config = repo_file(path);
        for expected in [
            "steamcmd=/data/steamcmd",
            "force_install_dir=/data/dst-dedicated-server",
            "backup=/data/backup",
            "mod_download_path=/data/mod",
            "persistent_storage_root=/data",
            "conf_dir=klei",
        ] {
            assert!(config.contains(expected), "{path} missing {expected}");
        }
    }
}

#[test]
fn mac_arm_docker_release_uses_root_context_and_data_volume() {
    for base in ["script/docker-build-mac", "scripts/docker-build-mac"] {
        let dockerfile = repo_file(&format!("{base}/Dockerfile"));
        assert!(
            dockerfile.contains("FROM --platform=linux/arm64 ubuntu:22.04"),
            "{base}/Dockerfile should pin the ARM64 image platform"
        );
        assert!(
            dockerfile.contains("VOLUME [\"/data\"]"),
            "{base}/Dockerfile should declare the persistent data volume"
        );
        assert!(
            dockerfile.contains(&format!(
                "COPY {base}/docker-entrypoint.sh /app/docker-entrypoint.sh"
            )),
            "{base}/Dockerfile should copy its entrypoint from a repository-root build context"
        );
        assert!(
            dockerfile.contains(&format!("COPY {base}/docker_dst_config /app/dst_config")),
            "{base}/Dockerfile should copy its DST config from a repository-root build context"
        );
        assert!(dockerfile.contains("COPY dist /app/dist"));
        assert!(dockerfile.contains("COPY static /app/static"));

        let entrypoint = repo_file(&format!("{base}/docker-entrypoint.sh"));
        assert!(entrypoint.contains("set -e"));
        assert!(entrypoint.contains("DATA_DIR=\"/data\""));
        assert!(entrypoint.contains("APP_DIR=\"/app\""));
        assert!(entrypoint.contains("dataDir: \".\""));
        assert!(entrypoint.contains("cp -a \"$APP_DIR/dist/.\" \"$DATA_DIR/dist/\""));
        assert!(entrypoint.contains("cp -a \"$APP_DIR/static/.\" \"$DATA_DIR/static/\""));
        assert!(entrypoint.contains("-dir \"$data_dst_server\""));
        assert!(entrypoint.contains("cd \"$DATA_DIR\""));
        assert!(entrypoint.contains("exec \"$APP_DIR/dst-admin-rust\""));
        assert_no_legacy_docker_paths(&format!("{base}/docker-entrypoint.sh"), &entrypoint);

        let config = repo_file(&format!("{base}/docker_dst_config"));
        for expected in [
            "steamcmd=/data/steamcmd",
            "force_install_dir=/data/dst-dedicated-server",
            "backup=/data/backup",
            "mod_download_path=/data/mod",
            "persistent_storage_root=/data",
            "conf_dir=klei",
            "bin=2664",
        ] {
            assert!(
                config.contains(expected),
                "{base}/docker_dst_config missing {expected}"
            );
        }
        assert_no_legacy_docker_paths(&format!("{base}/docker_dst_config"), &config);

        let notes = repo_file(&format!("{base}/dst-mac-arm64-env-install.md"));
        assert!(notes.contains("RUST_TARGET=aarch64-unknown-linux-gnu ./build_linux.sh"));
        assert!(notes.contains("-dir /data/dst-dedicated-server"));
        assert_no_legacy_docker_paths(&format!("{base}/dst-mac-arm64-env-install.md"), &notes);
    }

    let readme = repo_file("scripts/docker-build-mac/README.md");
    assert!(readme.contains(
        "docker build --platform linux/arm64 -f scripts/docker-build-mac/Dockerfile -t dst-admin-rust-arm64:latest ."
    ));
    assert!(readme.contains("-v ~/dstsave:/data"));
    assert!(readme.contains("/data/dst-dedicated-server"));
    assert!(readme.contains("/data/dst-db"));
    assert!(readme.contains("/data/dst-admin-go.log"));
    assert_no_legacy_docker_paths("scripts/docker-build-mac/README.md", &readme);
}

#[cfg(unix)]
#[test]
fn docker_publish_script_forces_amd64_rust_binary_for_amd64_image() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bin_dir = temp.path().join("bin");
    fs::create_dir(&bin_dir).expect("create fake bin dir");

    write_executable(
        &temp.path().join("build_linux.sh"),
        r#"#!/bin/sh
printf '%s' "$RUST_TARGET" > observed-rust-target
touch dst-admin-rust
"#,
    );
    write_executable(
        &bin_dir.join("docker"),
        r#"#!/bin/sh
printf '%s\n' "$*" >> docker-calls
"#,
    );

    let mut command = Command::new("bash");
    command
        .arg(repo_path("docker_build.sh"))
        .arg("test-tag")
        .current_dir(temp.path())
        .env("RUST_TARGET", "aarch64-unknown-linux-gnu");
    prepend_path(&mut command, &bin_dir);
    let output = command.output().expect("run docker_build.sh");

    assert!(output.status.success());
    assert_eq!(
        fs::read_to_string(temp.path().join("observed-rust-target")).expect("target record"),
        "x86_64-unknown-linux-gnu"
    );
    let docker_calls = fs::read_to_string(temp.path().join("docker-calls")).expect("docker calls");
    assert!(docker_calls.contains("build --platform linux/amd64 -t yimuu/dst-panel:test-tag ."));
    assert!(docker_calls.contains("push yimuu/dst-panel:test-tag"));
}

#[cfg(unix)]
#[test]
fn linux_release_script_fails_before_cargo_when_rust_target_is_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bin_dir = temp.path().join("bin");
    fs::create_dir(&bin_dir).expect("create fake bin dir");
    install_fake_rust_tools_without_target(&bin_dir, "x86_64-apple-darwin");

    let mut command = Command::new("sh");
    command
        .arg(repo_path("build_linux.sh"))
        .current_dir(temp.path())
        .env("RUST_TARGET", "x86_64-unknown-linux-gnu");
    prepend_path(&mut command, &bin_dir);
    let output = command.output().expect("run build_linux.sh");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Rust target 'x86_64-unknown-linux-gnu' is not installed"));
    assert!(
        !temp
            .path()
            .join("target/x86_64-unknown-linux-gnu/release/dst-admin-rust")
            .exists(),
        "cargo should not run before the target preflight passes"
    );
}

#[cfg(unix)]
#[test]
fn windows_release_script_fails_before_cargo_when_rust_target_is_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bin_dir = temp.path().join("bin");
    fs::create_dir(&bin_dir).expect("create fake bin dir");
    install_fake_rust_tools_without_target(&bin_dir, "x86_64-apple-darwin");

    let mut command = Command::new("sh");
    command
        .arg(repo_path("build_window.sh"))
        .current_dir(temp.path());
    prepend_path(&mut command, &bin_dir);
    let output = command.output().expect("run build_window.sh");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Rust target 'x86_64-pc-windows-gnu' is not installed"));
    assert!(
        !temp
            .path()
            .join("target/x86_64-pc-windows-gnu/release/dst-admin-rust.exe")
            .exists(),
        "cargo should not run before the target preflight passes"
    );
}

#[test]
fn release_build_scripts_fail_fast_for_missing_cross_targets() {
    let linux_script = repo_file("build_linux.sh");
    assert!(linux_script.contains("x86_64-unknown-linux-gnu"));
    assert!(
        linux_script.contains("rustup target list --installed"),
        "Linux build script should check that the requested Rust target is installed"
    );

    let windows_script = repo_file("build_window.sh");
    assert!(windows_script.contains("x86_64-pc-windows-gnu"));
    assert!(
        windows_script.contains("rustup target list --installed"),
        "Windows build script should check that the requested Rust target is installed"
    );
    assert!(
        windows_script.contains("x86_64-w64-mingw32-gcc"),
        "Windows GNU build script should check for the MinGW linker"
    );

    let readme = repo_file("README.md");
    assert!(
        readme.contains("./build_linux.sh"),
        "README should point release users at the script that copies ./dst-admin-rust"
    );
    assert!(readme.contains("LINUX_LINKER"));
    assert!(readme.contains("x86_64-pc-windows-gnu"));
    assert!(readme.contains("x86_64-w64-mingw32-gcc"));

    let readme_en = repo_file("README-EN.md");
    assert!(
        readme_en.contains("./build_linux.sh"),
        "English README should point release users at the script that copies ./dst-admin-rust"
    );
    assert!(readme_en.contains("LINUX_LINKER"));
    assert!(readme_en.contains("x86_64-pc-windows-gnu"));
    assert!(readme_en.contains("x86_64-w64-mingw32-gcc"));
}

#[cfg(unix)]
#[test]
fn linux_release_script_fails_before_cargo_when_cross_linker_is_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bin_dir = temp.path().join("bin");
    fs::create_dir(&bin_dir).expect("create fake bin dir");
    install_fake_rust_tools(&bin_dir, "aarch64-unknown-linux-gnu", "x86_64-apple-darwin");

    let mut command = Command::new("sh");
    command
        .arg(repo_path("build_linux.sh"))
        .current_dir(temp.path())
        .env("RUST_TARGET", "aarch64-unknown-linux-gnu");
    prepend_path(&mut command, &bin_dir);
    let output = command.output().expect("run build_linux.sh");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Linux linker 'aarch64-linux-gnu-gcc' is required"));
    assert!(
        !temp
            .path()
            .join("target/aarch64-unknown-linux-gnu/release/dst-admin-rust")
            .exists(),
        "cargo should not run before the linker preflight passes"
    );
}

#[cfg(unix)]
#[test]
fn linux_release_script_uses_matching_cross_linker_when_available() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bin_dir = temp.path().join("bin");
    fs::create_dir(&bin_dir).expect("create fake bin dir");
    install_fake_rust_tools(&bin_dir, "aarch64-unknown-linux-gnu", "x86_64-apple-darwin");
    write_executable(
        &bin_dir.join("aarch64-linux-gnu-gcc"),
        "#!/bin/sh\nexit 0\n",
    );

    let mut command = Command::new("sh");
    command
        .arg(repo_path("build_linux.sh"))
        .current_dir(temp.path())
        .env("RUST_TARGET", "aarch64-unknown-linux-gnu");
    prepend_path(&mut command, &bin_dir);
    let output = command.output().expect("run build_linux.sh");

    assert!(output.status.success());
    assert!(temp.path().join("dst-admin-rust").exists());
}

#[cfg(unix)]
#[test]
fn linux_release_script_passes_custom_linker_to_cargo() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bin_dir = temp.path().join("bin");
    fs::create_dir(&bin_dir).expect("create fake bin dir");
    install_fake_rust_tools(&bin_dir, "aarch64-unknown-linux-gnu", "x86_64-apple-darwin");
    write_executable(&bin_dir.join("custom-linux-linker"), "#!/bin/sh\nexit 0\n");

    let mut command = Command::new("sh");
    command
        .arg(repo_path("build_linux.sh"))
        .current_dir(temp.path())
        .env("RUST_TARGET", "aarch64-unknown-linux-gnu")
        .env("LINUX_LINKER", "custom-linux-linker")
        .env("EXPECTED_AARCH64_LINKER", "custom-linux-linker");
    prepend_path(&mut command, &bin_dir);
    let output = command.output().expect("run build_linux.sh");

    assert!(output.status.success());
    assert!(temp.path().join("dst-admin-rust").exists());
}

#[cfg(unix)]
#[test]
fn windows_release_script_fails_before_cargo_when_mingw_linker_is_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bin_dir = temp.path().join("bin");
    fs::create_dir(&bin_dir).expect("create fake bin dir");
    install_fake_rust_tools(&bin_dir, "x86_64-pc-windows-gnu", "x86_64-apple-darwin");

    let mut command = Command::new("sh");
    command
        .arg(repo_path("build_window.sh"))
        .current_dir(temp.path());
    prepend_path(&mut command, &bin_dir);
    let output = command.output().expect("run build_window.sh");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("MinGW linker x86_64-w64-mingw32-gcc is required"));
    assert!(
        !temp
            .path()
            .join("target/x86_64-pc-windows-gnu/release/dst-admin-rust.exe")
            .exists(),
        "cargo should not run before the linker preflight passes"
    );
}
