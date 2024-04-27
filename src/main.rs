use std::{
    env,
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

struct Route {
    location: PathBuf,
    extension: Option<String>,
    transformers: Vec<Arc<dyn Transformer>>,
    destination: PathBuf,
}

impl Route {
    fn start(location: PathBuf) -> Self {
        Self {
            location,
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

    fn to(mut self, location: PathBuf) -> Self {
        self.destination = location;
        self
    }
}

struct EIP {
    routes: Vec<Route>,
}

impl EIP {
    fn new() -> Self {
        Self { routes: Vec::new() }
    }
    fn routes(mut self, r: Vec<Route>) -> Self {
        self.routes = r;
        self
    }

    fn run(&self) -> io::Result<()> {
        for route in &self.routes {
            let extension_filter = &route.extension.clone().unwrap_or_else(|| "*".to_string());
            let destination = &route.destination;

            for entry in WalkDir::new(&route.location)
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

                for transformer in &route.transformers {
                    transformed_data = transformer.transform(&transformed_data.data)?;
                }

                let output_path = destination.join(entry.file_name());
                let mut output_file = BufWriter::new(File::create(output_path)?);
                output_file.write_all(&transformed_data.data)?;
            }
        }

        Ok(())
    }
}

fn main() -> io::Result<()> {
    let transformer = Arc::new(FileTransformer::new(1024));
    let current_dir = env::current_dir().expect("failed to get current directory");

    let input_dir = current_dir.join("input");
    let output_dir = current_dir.join("output");

    EIP::new()
        .routes(vec![Route::start(input_dir)
            .extension("csv")
            .stream()
            .then(transformer)
            .to(output_dir)])
        .run()
}
