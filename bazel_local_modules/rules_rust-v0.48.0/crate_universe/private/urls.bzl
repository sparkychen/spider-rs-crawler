"""A file containing urls and associated sha256 values for cargo-bazel binaries

This file is auto-generated for each release to match the urls and sha256s of
the binaries produced for it.
"""

# Example:
# {
#     "x86_64-unknown-linux-gnu": "https://domain.com/downloads/cargo-bazel-x86_64-unknown-linux-gnu",
#     "x86_64-apple-darwin": "https://domain.com/downloads/cargo-bazel-x86_64-apple-darwin",
#     "x86_64-pc-windows-msvc": "https://domain.com/downloads/cargo-bazel-x86_64-pc-windows-msvc",
# }
CARGO_BAZEL_URLS = {
  "aarch64-apple-darwin": "https://github.com/bazelbuild/rules_rust/releases/download/0.48.0/cargo-bazel-aarch64-apple-darwin",
  "aarch64-pc-windows-msvc": "https://github.com/bazelbuild/rules_rust/releases/download/0.48.0/cargo-bazel-aarch64-pc-windows-msvc.exe",
  "aarch64-unknown-linux-gnu": "https://github.com/bazelbuild/rules_rust/releases/download/0.48.0/cargo-bazel-aarch64-unknown-linux-gnu",
  "x86_64-apple-darwin": "https://github.com/bazelbuild/rules_rust/releases/download/0.48.0/cargo-bazel-x86_64-apple-darwin",
  "x86_64-pc-windows-gnu": "https://github.com/bazelbuild/rules_rust/releases/download/0.48.0/cargo-bazel-x86_64-pc-windows-gnu.exe",
  "x86_64-pc-windows-msvc": "https://github.com/bazelbuild/rules_rust/releases/download/0.48.0/cargo-bazel-x86_64-pc-windows-msvc.exe",
  "x86_64-unknown-linux-gnu": "https://github.com/bazelbuild/rules_rust/releases/download/0.48.0/cargo-bazel-x86_64-unknown-linux-gnu",
  "x86_64-unknown-linux-musl": "https://github.com/bazelbuild/rules_rust/releases/download/0.48.0/cargo-bazel-x86_64-unknown-linux-musl"
}

# Example:
# {
#     "x86_64-unknown-linux-gnu": "1d687fcc860dc8a1aa6198e531f0aee0637ed506d6a412fe2b9884ff5b2b17c0",
#     "x86_64-apple-darwin": "0363e450125002f581d29cf632cc876225d738cfa433afa85ca557afb671eafa",
#     "x86_64-pc-windows-msvc": "f5647261d989f63dafb2c3cb8e131b225338a790386c06cf7112e43dd9805882",
# }
CARGO_BAZEL_SHA256S = {
  "aarch64-apple-darwin": "9f5f4399e099816fe74c57c33a7b67fec37aed3bab263dd0d845ea295a1e2cf3",
  "aarch64-pc-windows-msvc": "a9656dadc81dfa1f2787b2380a655986470d96fd71c30216372e1098d12b40da",
  "aarch64-unknown-linux-gnu": "966df8d69ef7f8c7b80cd85fc1d26b7bfbec21c150ce51cf33c75404729e30ca",
  "x86_64-apple-darwin": "192437eef76a58af02be4d77ec58d2e2653a624812c3e9d65ad441d64eec5fc1",
  "x86_64-pc-windows-gnu": "6ded4199a92a809ca6ec185da9daecae7ec6637ddd22daab6b69d66a89380aee",
  "x86_64-pc-windows-msvc": "415e461f9e88e6455861ebc3a520eb130af771522ed92eacbad42a7051ebd9df",
  "x86_64-unknown-linux-gnu": "a93667a783955811fc735e0da578f75c975fe384fc36d7633b5249fd7f1a2984",
  "x86_64-unknown-linux-musl": "fd4bedec5fb5bda40fd2f016bc5a4a788c16e963712a22f57efaece41ebedd05"
}

# Example:
# Label("//crate_universe:cargo_bazel_bin")
CARGO_BAZEL_LABEL = Label("//crate_universe:cargo_bazel_bin")
