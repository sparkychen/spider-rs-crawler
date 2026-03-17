http_archive(
    name = "rules_pkg",
    sha256 = "8f9ee2dc10c1ae514ee599a8b42ed99fa262b757058f65ad3c384289ff70c4b8",
    urls = ["https://github.com/bazelbuild/rules_pkg/releases/download/0.12.1/rules_pkg-0.12.1.tar.gz"],
)

# 加载rules_pkg规则
load("@rules_pkg//:deps.bzl", "rules_pkg_dependencies")
rules_pkg_dependencies()

load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")

# 1. 核心依赖：bazel_skylib
http_archive(
    name = "bazel_skylib",
    sha256 = "cd55a062e763b9349921f0f5db8c3933288dc8ba4f76dd9416aac68acee3cb94",
    urls = ["https://github.com/bazelbuild/bazel-skylib/releases/download/1.5.0/bazel-skylib-1.5.0.tar.gz"],
)

# 2. 修复pkg依赖：rules_pkg（替代bazel_tools的pkg）
http_archive(
    name = "rules_pkg",
    sha256 = "8f9ee2dc10c1ae514ee599a8b42ed99fa262b757058f65ad3c384289ff70c4b8",
    urls = ["https://github.com/bazelbuild/rules_pkg/releases/download/0.12.1/rules_pkg-0.12.1.tar.gz"],
)

# 3. rules_rust 0.42.0
http_archive(
    name = "rules_rust",
    sha256 = "5d3d58549e851c94d706bd6fde9c767d5dfbfce092ddd424dee97e5ef7c821ff",
    urls = ["https://gh-proxy.org/https://github.com/bazelbuild/rules_rust/releases/download/0.42.0/rules_rust-v0.42.0.tar.gz"],
)

# 4. 初始化依赖
load("@bazel_skylib//:workspace.bzl", "bazel_skylib_workspace")
bazel_skylib_workspace()

load("@rules_rust//rust:repositories.bzl", "rules_rust_dependencies", "rust_register_toolchains")
rules_rust_dependencies()

# 5. 注册Rust工具链
rust_register_toolchains(
    edition = "2021",
    versions = ["1.94.0"],
)
