use std::{
    fs::File,
    io::{self, BufWriter, Read, Write},
    path::PathBuf,
    sync::Arc,
};

use walkdir::WalkDir;

struct TransformedData {
    data: Vec<u8>,
}

trait Transformer {
    fn transform(&self, input: &[u8]) -> io::Result<TransformedData>;
}

struct FileTransformer {
    chunk_size: usize,
}

impl FileTransformer {
    fn new(chunk_size: usize) -> Self {
        FileTransformer { chunk_size }
    }
}

impl Transformer for FileTransformer {
    fn transform(&self, input: &[u8]) -> io::Result<TransformedData> {
        let mut data = Vec::new();
        let mut start = 0;
        while start < input.len() {
            let end = std::cmp::min(start + self.chunk_size, input.len());
            data.extend_from_slice(&input[start..end]);
            start = end;
        }
        Ok(TransformedData { data })
    }
}

struct EipBuilder {
    location: PathBuf,
    extension: Option<String>,
    transformers: Vec<Arc<dyn Transformer>>,
    destination: PathBuf,
}

impl EipBuilder {
    fn start(location: &str) -> Self {
        Self {
            location: PathBuf::from(location),
            extension: None,
            transformers: Vec::new(),
            destination: PathBuf::new(),
        }
    }

    fn extension(mut self, ext: &str) -> Self {
        self.extension = Some(ext.to_string());
        self
    }

    fn stream(self) -> Self {
        self
    }

    fn then(mut self, transformer: Arc<dyn Transformer>) -> Self {
        self.transformers.push(transformer);
        self
    }

    fn to(mut self, location: &str) -> Self {
        self.destination = PathBuf::from(location);
        self
    }

    fn run(&self) -> io::Result<()> {
        let extension_filter = self.extension.clone().unwrap_or_else(|| "*".to_string());
        let destination = &self.destination;

        for entry in WalkDir::new(&self.location)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .filter(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == extension_filter.trim_start_matches('*'))
                    .unwrap_or(false)
            })
        {
            let mut file = File::open(entry.path())?;
            let mut contents = Vec::new();
            file.read_to_end(&mut contents)?;

            let mut transformed_data = TransformedData { data: contents };

            for transformer in &self.transformers {
                transformed_data = transformer.transform(&transformed_data.data)?;
            }

            let output_path = destination.join(entry.file_name());
            let mut output_file = BufWriter::new(File::create(output_path)?);
            output_file.write_all(&transformed_data.data)?;
        }

        Ok(())
    }
}

fn main() -> io::Result<()> {
    let transformer = Arc::new(FileTransformer::new(1024));

    EipBuilder::start("/home/jd/development/rust/toy_eip/input")
        .extension("csv")
        .stream()
        .then(transformer)
        .to("/home/jd/development/rust/toy_eip/output")
        .run()
}
