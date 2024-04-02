task :default => [:check, :build]
task :all => [:clean, :default]

$rust_log = "plugin_host=debug"
$wasi_out = "wasi.txt"

desc "Clean project"
task :clean do |t|
    title t.name
    Rake.sh "cargo +stable clean"
end

desc "Check project"
task :check do |t|
    title t.name
    Rake.sh "cargo +stable clippy"
    Rake.sh "cargo +stable test"
    check_wasi_output($wasi_out, "Hello WASI!")
    Rake.sh "cargo +stable fmt"
end

desc "Build project"
task :build do |t|
    title t.name
    Rake.sh "cargo +stable build"
end

desc "Dev project"
task :dev do |t|
    title t.name
    cd "tests" do
        Rake.sh "rake"
    end
    Rake.sh "RUST_LOG=#{$rust_log} RUST_TEST_THREADS=1 cargo +stable test"
    check_wasi_output($wasi_out, "Hello WASI!")
end

def check_wasi_output(path, expected)
    if File.exist?(path) then
        begin
            actual = File.read(path).chomp
            raise "wasi output invalid: #{actual}" if !actual.eql?(expected)
        ensure
            FileUtils.remove(path)
        end
    else
        raise "wasi output missing"
    end
end

def title(t)
    puts "\n#{'='*40}\n\s#{t.upcase}\n#{'='*40}\n\n"
end
