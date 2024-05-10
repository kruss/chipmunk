require 'fileutils'

task :default => [:build]
task :all => [:clean, :check, :test]

task :setup do
    system("rustup target add wasm32-wasi")
end

desc "Clean WASI plugin"
task :clean do |t|
    title t.name
    Rake.sh "cargo +stable clean"
    Rake.sh "cargo +nightly clean"
    FileUtils.remove("plugin.wasm") if File.exist?("plugin.wasm")
end

desc "Check WASI plugin"
task :check do |t|
    title t.name
    Rake.sh "cargo +stable clippy --lib"
    Rake.sh "cargo +stable fmt --lib"
    Rake.sh "cargo +nightly clippy --bin plugin"
    Rake.sh "cargo +nightly fmt --bin plugin"
end

desc "Test WASI plugin"
task :test => :build do |t|
    title t.name
    Rake.sh "cargo +stable test --lib"
end

desc "Build WASI plugin"
task :build => :setup do |t|
    title t.name
    Rake.sh "cargo +stable build --lib"
    Rake.sh "cargo +nightly rustc --bin plugin --release --target wasm32-wasi -- -Z wasi-exec-model=reactor"
    cd "target/wasm32-wasi/release" do
        FileUtils.remove("plugin.wat") if File.exist?("plugin.wat")
        system("wasm2wat plugin.wasm >> plugin.wat")
    end
end

def title(t)
    puts "\n#{'='*40}\n\s#{t.upcase}\n#{'='*40}\n\n"
end