pub mod dlt;
pub mod source;

#[cfg(test)]
mod tests {
    use crate::plugin::dlt::*;
    use crate::plugin::source::*;
    use parsers::dlt::DltParser;
    use plugin_host::PluginFactory;
    use processor::export::export_raw;
    use sources::{
        binary::raw::BinaryByteSource, producer::MessageProducer, DEFAULT_MIN_BUFFER_SPACE,
        DEFAULT_READER_CAPACITY,
    };
    use std::{env, fs, fs::File, io::BufReader, path::PathBuf};
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;

    /**
     * Runs the native source and parser for an input file.
     *
     * To run the test with default input file use commandline:
     * cargo test --release -- --nocapture --ignored run_native
     *
     * To run the test with specific input file use commandline:
     * INPUT=<path-to-file> cargo test --release -- --nocapture --ignored run_native
     *
     * To produce a flamegraph run with command-line:
     * sudo CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --root --release --unit-test -- tests::test_topologize_skeleton --test --nocapture --ignored run_native
     */
    #[ignore]
    #[tokio::test]
    async fn run_native() {
        let tmp_dir = TempDir::new().unwrap();
        let mut out_path = tmp_dir.path().to_owned();
        out_path.push("test.out");

        let in_path;
        if let Ok(path) = env::var("INPUT") {
            in_path = PathBuf::from(path);
        } else {
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("../indexer_cli/test/dlt/test.dlt");
            in_path = path;
        };

        let dlt_file = File::open(in_path).unwrap();
        let reader = BufReader::new(&dlt_file);
        let source = BinaryByteSource::new(reader);
        let dlt_parser = DltParser::new(None, None, None, true);
        let mut dlt_msg_producer = MessageProducer::new(dlt_parser, source, None);
        let cancel = CancellationToken::new();

        export_raw(
            Box::pin(dlt_msg_producer.as_stream()),
            &out_path,
            &vec![],
            false,
            false,
            &cancel,
        )
        .await
        .expect("export_raw");
    }

    /**
     * Runs the source plugin for an input file.
     *
     * To run the test with default input file use commandline:
     * cargo test --release -- --nocapture --ignored run_src_plugin
     *
     * To run the test with specific input file use commandline:
     * INPUT=<path-to-file> cargo test --release -- --nocapture --ignored run_src_plugin
     *
     * To produce a flamegraph run with command-line:
     * sudo CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --root --release --unit-test -- tests::test_topologize_skeleton --test --nocapture --ignored run_src_plugin
     */
    #[ignore]
    #[tokio::test]
    async fn run_src_plugin() {
        let tmp_dir = TempDir::new().unwrap();
        let mut out_path = tmp_dir.path().to_owned();
        out_path.push("test.out");

        let mut in_path;
        if let Ok(path) = env::var("INPUT") {
            in_path = PathBuf::from(path);
        } else {
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("../indexer_cli/test/dlt/test.dlt");
            in_path = path;
        };

        // TODO temp !
        let temp_path = PathBuf::from("temp.dlt");
        fs::copy(in_path.clone(), temp_path.clone()).expect("copy");
        in_path = temp_path.clone();
        // ---

        let source_plugin_factory = SourcePluginFactory::new();
        let source_plugin = source_plugin_factory.create(0).unwrap();
        let source = ByteSourceProxy::new(
            source_plugin,
            &in_path,
            DEFAULT_READER_CAPACITY,
            DEFAULT_MIN_BUFFER_SPACE,
        );
        let dlt_parser = DltParser::new(None, None, None, true);
        let mut dlt_msg_producer = MessageProducer::new(dlt_parser, source, None);
        let cancel = CancellationToken::new();

        export_raw(
            Box::pin(dlt_msg_producer.as_stream()),
            &out_path,
            &vec![],
            false,
            false,
            &cancel,
        )
        .await
        .expect("export_raw");

        fs::remove_file(temp_path).expect("remove file"); // TODO temp !
    }

    /**
     * Runs the parser plugin for an input file.
     *
     * To run the test with default input file use commandline:
     * cargo test --release -- --nocapture --ignored run_dlt_plugin
     *
     * To run the test with specific input file use commandline:
     * INPUT=<path-to-file> cargo test --release -- --nocapture --ignored run_dlt_plugin
     *
     * To produce a flamegraph run with command-line:
     * sudo CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --root --release --unit-test -- tests::test_topologize_skeleton --test --nocapture --ignored run_dlt_plugin
     */
    #[ignore]
    #[tokio::test]
    async fn run_dlt_plugin() {
        let tmp_dir = TempDir::new().unwrap();
        let mut out_path = tmp_dir.path().to_owned();
        out_path.push("test.out");

        let in_path;
        if let Ok(path) = env::var("INPUT") {
            in_path = PathBuf::from(path);
        } else {
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("../indexer_cli/test/dlt/test.dlt");
            in_path = path;
        };

        let dlt_file = File::open(in_path).unwrap();
        let reader = BufReader::new(&dlt_file);
        let source = BinaryByteSource::new(reader);
        let dlt_plugin_factory = DltPluginFactory::new();
        let dlt_plugin = dlt_plugin_factory.create(0).unwrap();
        let dlt_parser = DltParserProxy::new(dlt_plugin, true);
        let mut dlt_msg_producer = MessageProducer::new(dlt_parser, source, None);
        let cancel = CancellationToken::new();

        export_raw(
            Box::pin(dlt_msg_producer.as_stream()),
            &out_path,
            &vec![],
            false,
            false,
            &cancel,
        )
        .await
        .expect("export_raw");
    }

    /**
     * Runs the source and parser plugin for an input file.
     *
     * To run the test with default input file use commandline:
     * cargo test --release -- --nocapture --ignored run_src_and_dlt_plugin
     *
     * To run the test with specific input file use commandline:
     * INPUT=<path-to-file> cargo test --release -- --nocapture --ignored run_src_and_dlt_plugin
     *
     * To produce a flamegraph run with command-line:
     * sudo CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --root --release --unit-test -- tests::test_topologize_skeleton --test --nocapture --ignored run_src_and_dlt_plugin
     */
    #[ignore]
    #[tokio::test]
    async fn run_src_and_dlt_plugin() {
        let tmp_dir = TempDir::new().unwrap();
        let mut out_path = tmp_dir.path().to_owned();
        out_path.push("test.out");

        let mut in_path;
        if let Ok(path) = env::var("INPUT") {
            in_path = PathBuf::from(path);
        } else {
            let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            path.push("../indexer_cli/test/dlt/test.dlt");
            in_path = path;
        };

        // TODO temp !
        let temp_path = PathBuf::from("temp.dlt");
        fs::copy(in_path.clone(), temp_path.clone()).expect("copy");
        in_path = temp_path.clone();
        // ---

        let source_plugin_factory = SourcePluginFactory::new();
        let source_plugin = source_plugin_factory.create(0).unwrap();
        let source = ByteSourceProxy::new(
            source_plugin,
            &in_path,
            DEFAULT_READER_CAPACITY,
            DEFAULT_MIN_BUFFER_SPACE,
        );
        let dlt_plugin_factory = DltPluginFactory::new();
        let dlt_plugin = dlt_plugin_factory.create(1).unwrap();
        let dlt_parser = DltParserProxy::new(dlt_plugin, true);
        let mut dlt_msg_producer = MessageProducer::new(dlt_parser, source, None);
        let cancel = CancellationToken::new();

        export_raw(
            Box::pin(dlt_msg_producer.as_stream()),
            &out_path,
            &vec![],
            false,
            false,
            &cancel,
        )
        .await
        .expect("export_raw");

        fs::remove_file(temp_path).expect("remove file"); // TODO temp !
    }
}
