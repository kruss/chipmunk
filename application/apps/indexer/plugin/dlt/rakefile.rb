require 'fileutils'

task :default => [:build]
task :all => [:clean, :check, :test]

task :setup do
    system("rustup target add wasm32-unknown-unknown")
    #system("cargo install wasm-gc")
end

desc "Clean WASM plugin"
task :clean do |t|
    title t.name
    Rake.sh "cargo +stable clean"
end

desc "Check WASM plugin"
task :check do |t|
    title t.name
    Rake.sh "cargo +stable clippy"
    Rake.sh "cargo +stable fmt"
end

desc "Test WASM plugin"
task :test => :build do |t|
    title t.name
    Rake.sh "cargo +stable test"
end

desc "Build WASM plugin"
task :build => :setup do |t|
    title t.name
    Rake.sh "cargo +stable build --release --target wasm32-unknown-unknown"
    cd "target/wasm32-unknown-unknown/release" do
        FileUtils.remove("plugin.wat") if File.exist?("plugin.wat")
        system("wasm2wat plugin.wasm >> plugin.wat")
        #system("wasm-gc plugin.wasm")
    end 
end

def title(t)
    puts "\n#{'='*40}\n\s#{t.upcase}\n#{'='*40}\n\n"
end
