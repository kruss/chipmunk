require 'fileutils'

task :default => ['wasm:build', 'wasi:build']
task :clean => ['wasm:clean', 'wasi:clean']
task :all => ['wasm:all', 'wasi:all']

namespace :wasm do
    task :default => [:build]
    task :all => [:clean, :check, :build]
    
    task :setup do
        system("rustup target add wasm32-unknown-unknown")
        #system("cargo install wasm-gc")
    end
    
    desc "Clean WASM plugin"
    task :clean do |t|
        title t.name
        cd "wasm" do
            Rake.sh "cargo +stable clean"
            FileUtils.remove("plugin.wasm") if File.exist?("plugin.wasm")
        end
    end
    
    desc "Check WASM plugin"
    task :check do |t|
        title t.name
        cd "wasm" do
            Rake.sh "cargo +stable clippy"
            Rake.sh "cargo +stable fmt"
        end
    end
    
    desc "Build WASM plugin"
    task :build => :setup do |t|
        title t.name
        cd "wasm" do
            Rake.sh "cargo +stable build --release --target wasm32-unknown-unknown"
            cd "target/wasm32-unknown-unknown/release" do
                FileUtils.remove("plugin.wat") if File.exist?("plugin.wat")
                system("wasm2wat plugin.wasm >> plugin.wat")
                #system("wasm-gc plugin.wasm")
                FileUtils.cp("plugin.wasm", "../../../")
            end
        end  
    end
end

namespace :wasi do
    task :default => [:build]
    task :all => [:clean, :check, :build]
    
    task :setup do
        system("rustup target add wasm32-wasi")
    end
    
    desc "Clean WASI plugin"
    task :clean do |t|
        title t.name
        cd "wasi" do
            Rake.sh "cargo +stable clean"
            FileUtils.remove("plugin.wasm") if File.exist?("plugin.wasm")
        end
    end
    
    desc "Check WASI plugin"
    task :check do |t|
        title t.name
        cd "wasi" do
            Rake.sh "cargo +stable clippy"
            Rake.sh "cargo +stable fmt"
        end
    end
    
    desc "Build WASI plugin"
    task :build => :setup do |t|
        title t.name
        cd "wasi" do
            Rake.sh "cargo +nightly rustc --release --target wasm32-wasi -- -Z wasi-exec-model=reactor"
            cd "target/wasm32-wasi/release" do
                FileUtils.remove("plugin.wat") if File.exist?("plugin.wat")
                system("wasm2wat plugin.wasm >> plugin.wat")
                FileUtils.cp("plugin.wasm", "../../../")
            end
        end  
    end
end

def title(t)
    puts "\n#{'='*40}\n\s#{t.upcase}\n#{'='*40}\n\n"
end
