# 启用Bzlmod兼容（可选，减少警告）
enable_bzlmod = False

# 新增 local_repository 规则，指向本地文件
local_repository(
    name = "bazel_features",
    path = "./bazel_local_modules/bazel_features-v1.21.0", # 替换为你的实际保存路径
)

# ========== 第一步：必须先加载 http_archive 函数（核心修复） ==========
load("@bazel_tools//tools/build_defs/repo:http.bzl", "http_archive")


# ========== 1. 基础依赖：bazel_skylib（所有规则的基础） ==========
http_archive(
    name = "bazel_skylib",
    sha256 = "cd55a062e763b9349921f0f5db8c3933288dc8ba4f76dd9416aac68acee3cb94",
    urls = ["https://gh-proxy.org/https://github.com/bazelbuild/bazel-skylib/releases/download/1.5.0/bazel-skylib-1.5.0.tar.gz"],
)

# ========== 2. 修复 rules_pkg（改用 0.15.0 稳定版，链接有效） ==========
#http_archive(
#    name = "rules_pkg",
#    sha256 = "706e77645016c510765ecc46e8c1f18444f0a81e8a191107e9367c91c93953a0",
#    urls = ["https://github.com/bazelbuild/rules_pkg/releases/download/0.15.0/rules_pkg-0.15.0.tar.gz"],
#)
local_repository(
    name = "rules_pkg",
    path = "./bazel_local_modules/rules_pkg-1.0.1/", # 替换为你的实际保存路径
)

local_repository(
    name = "platforms",
    path = "./bazel_local_modules/platforms-0.0.8/", # 替换为你的实际保存路径
)

http_archive(
    name = "build_bazel_apple_support",
    sha256 = "1c4031e72b456a048d8177f59a5581808c07585fa9e255c6f5fefb8752af7e40",
    url = "https://gh-proxy.org/https://github.com/bazelbuild/apple_support/releases/download/1.13.0/apple_support.1.13.0.tar.gz",
)
load(
    "@build_bazel_apple_support//lib:repositories.bzl",
    "apple_support_dependencies",
)
apple_support_dependencies()


http_archive(
    name = "aspect_bazel_lib",
    sha256 = "f2c1f91cc0a55f7a44c94b8a79974f21349b844075740c01045acaa49e731307",
    strip_prefix = "bazel-lib-1.40.3",
    url = "https://gh-proxy.org/https://github.com/aspect-build/bazel-lib/releases/download/v1.40.3/bazel-lib-v1.40.3.tar.gz",
)

load("@aspect_bazel_lib//lib:repositories.bzl", "aspect_bazel_lib_dependencies")
aspect_bazel_lib_dependencies()

http_archive(
    name = "rules_java",
    urls = [
        "https://gh-proxy.org/https://github.com/bazelbuild/rules_java/releases/download/7.1.0/rules_java-7.1.0.tar.gz",
    ],
    sha256 = "a37a4e5f63ab82716e5dd6aeef988ed8461c7a00b8e936272262899f587cd4e1",
)
load("@rules_java//java:repositories.bzl", "rules_java_dependencies", "rules_java_toolchains")
rules_java_dependencies()
rules_java_toolchains()

http_archive(
    name = "platforms",
    urls = [
        "https://mirror.bazel.build/github.com/bazelbuild/platforms/releases/download/0.0.8/platforms-0.0.8.tar.gz",
        "https://gh-proxy.org/https://github.com/bazelbuild/platforms/releases/download/0.0.8/platforms-0.0.8.tar.gz",
    ],
    sha256 = "8150406605389ececb6da07cbcb509d5637a3ab9a24bc69b1101531367d89d74",
)

# ========== 3. 加载 rules_pkg 依赖（需在 bazel_skylib 之后） ==========
# 加载rules_pkg规则
load("@rules_pkg//:deps.bzl", "rules_pkg_dependencies")
rules_pkg_dependencies()

# ========== 4. rules_rust 0.42.0（适配 Bazel 7.x + Rust 2021） ==========
http_archive(
    name = "rules_rust",
    sha256 = "5d3d58549e851c94d706bd6fde9c767d5dfbfce092ddd424dee97e5ef7c821ff",
    urls = ["https://gh-proxy.org/https://github.com/bazelbuild/rules_rust/releases/download/0.42.0/rules_rust-v0.42.0.tar.gz"],
)

# ========== 5. 初始化依赖 + 注册 Rust 工具链 ==========
load("@bazel_skylib//:workspace.bzl", "bazel_skylib_workspace")
bazel_skylib_workspace()

load("@rules_rust//rust:repositories.bzl", "rules_rust_dependencies", "rust_register_toolchains")
rules_rust_dependencies()

# 注册 Rust 工具链（匹配你的代码 edition 2021）
rust_register_toolchains(
    edition = "2021",
    versions = ["1.94.0"],
)

