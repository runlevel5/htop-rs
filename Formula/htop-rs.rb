class HtopRs < Formula
  desc "Interactive process viewer, a Rust port of htop"
  homepage "https://github.com/runlevel5/htop-rs"
  url "https://github.com/runlevel5/htop-rs/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "168f21f75c5df1068e4bdf1839473e07a56b48d459c7b3dedf4d78d847601818"
  license "GPL-2.0-or-later"
  head "https://github.com/runlevel5/htop-rs.git", branch: "main"

  depends_on "rust" => :build

  def install
    # Only the viewer; the repository also builds developer tools and benchmarks
    system "cargo", "install", "--bin", "htop-rs", *std_cargo_args
  end

  test do
    assert_match "htop #{version}", shell_output("#{bin}/htop-rs --version")
  end
end
