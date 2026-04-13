## Feature: conduit release workflow
**Status**: In Progress

### Goals
- 在推送版本 tag（`v*`）时，自动构建 `conduit` 的发布产物并上传到 GitHub Releases。
- 首版范围聚焦 Linux（musl）+ macOS，覆盖 amd64 与 arm64。
- 保持实现尽量简单：延续现有 Cargo 构建方式，不额外引入复杂发布框架。

### Stages
1. **Stage 1**: 定义发布 workflow 结构
   - **Status**: Complete
   - 已新增 `.github/workflows/release.yml`
   - 采用 tag 触发，只在 `v*` 推送时发布
   - 先执行 `cargo fmt -- --check`、`cargo clippy --all-targets --all-features`、`cargo test`
   - **Validation**: workflow trigger、permissions、job 依赖关系清晰

2. **Stage 2**: 构建四个目标产物
   - **Status**: Code Complete, Remote Validation Pending
   - Linux amd64: `x86_64-unknown-linux-musl`
   - Linux arm64: `aarch64-unknown-linux-musl`
   - macOS amd64: `x86_64-apple-darwin`
   - macOS arm64: `aarch64-apple-darwin`
   - 复用二进制名 `conduit`
   - Linux musl targets 使用 `cross` 处理构建，其余目标用 `cargo build --release --target ...`
   - **Validation**: 每个 target 都能产出对应 `target/<triple>/release/conduit`

3. **Stage 3**: 打包并上传到 GitHub Release
   - **Status**: Code Complete, Remote Validation Pending
   - 每个 target 产物打包为 `tar.gz`
   - 资产命名采用 `conduit-<tag>-<target>.tar.gz`
   - matrix job 先上传 workflow artifacts，最后统一发布到 GitHub Release
   - **Validation**: release 页面包含四个 tar.gz 资产，下载后可解压出 `conduit`

4. **Stage 4**: 更新文档
   - **Status**: Complete
   - 已在 `README.md` 中补充 release 发布说明
   - 已说明 tag 触发方式、支持 target、资产命名规则
   - **Validation**: README 与 workflow 行为保持一致
