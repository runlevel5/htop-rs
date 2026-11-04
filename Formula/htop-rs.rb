class HtopRs < Formula
  desc "Interactive process viewer, a Rust port of htop"
  homepage "https://github.com/runlevel5/htop-rs"
  url "https://github.com/runlevel5/htop-rs/archive/refs/tags/v0.2.1.tar.gz"
  sha256 "d56b17d4c94f7e63c4ff358c61456cafbc467e429c5726f81192f0d11a298bf9"
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
