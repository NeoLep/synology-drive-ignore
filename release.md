## GitHub Release

项目已包含 GitHub Actions 发布配置：[.github/workflows/release.yml](/Users/leep/Desktop/Workspace2/synology-drive-ignore/.github/workflows/release.yml)。

创建并推送版本标签后会自动构建 macOS DMG 和 Windows NSIS 安装包，并上传到一个草稿 Release：

```sh
git tag v0.1.0
git push origin v0.1.0
```

也可以在 GitHub Actions 页面手动运行 `Release` workflow。